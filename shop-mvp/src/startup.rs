// =============================================================================
// 🚀 Startup — Bootstrap logic extras din main.rs
// =============================================================================
// Conține toată logica de inițializare: config, DB, LEGO assembly, server.
// main.rs rămîne doar punctul de intrare.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use sqlx::PgPool;
use tera::Tera;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use rust_marketplace_products::{PgProductRepo, ProductRepo};
use rust_auth::PgAuthRepo;
use rust_cart::PgCartRepo;
use rust_marketplace_orders::PgOrderRepo;
use rust_payment::{PaymentRepo, StripePayment, MockPayment, RetryPayment};

use crate::boundary;
use crate::render::RenderService;
use crate::state::*;

// ============================================================================
// Config — încărcată o singură dată la startup
// ============================================================================

pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub stripe_secret: String,
    pub site_url: String,
    pub max_qty: i32,
    pub port: String,
    pub mock_payment: bool,
    pub stripe_webhook_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
            .unwrap_or_else(|_| {
                tracing::warn!("⚠️  STRIPE_WEBHOOK_SECRET ne setat — webhook-urile Stripe NU vor fi verificate!");
                String::new()
            });
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:123123@localhost:5432/test".into()),
            jwt_secret: std::env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set in environment"),
            stripe_secret: std::env::var("STRIPE_SECRET_KEY")
                .expect("STRIPE_SECRET_KEY must be set in environment"),
            site_url: std::env::var("SITE_URL")
                .unwrap_or_else(|_| "http://localhost:3001".into())
                .trim_end_matches('/')
                .to_string(),
            max_qty: std::env::var("MAX_QTY_PER_PRODUCT")
                .unwrap_or_else(|_| "999".into())
                .parse()
                .unwrap_or(999),
            port: std::env::var("PORT").unwrap_or_else(|_| "3001".into()),
            mock_payment: std::env::var("MOCK_PAYMENT").as_deref() == Ok("true"),
            stripe_webhook_secret,
        }
    }
}

// ============================================================================
// Panic hook
// ============================================================================

pub fn init_panic_hook() {
    let hook_panic_log = std::path::PathBuf::from("logs/panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        let location = info.location().map(|l| l.to_string()).unwrap_or_default();
        let backtrace = std::backtrace::Backtrace::capture();
        let panic_log = format!(
            "=== PANIC ===\n{}\nLocation: {}\nBacktrace:\n{:?}\n==============",
            msg, location, backtrace
        );
        let _ = std::fs::OpenOptions::new()
            .create(true).append(true).open(&hook_panic_log)
            .and_then(|mut f| std::io::Write::write_all(&mut f, format!("{}\n", panic_log).as_bytes()));
        eprintln!("{}", panic_log);
    }));
}

// ============================================================================
// Logging
// ============================================================================

pub fn init_logging() {
    let _ = std::fs::create_dir_all("logs");
    let file_appender = tracing_appender::rolling::daily("logs", "shop-mvp.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            if std::env::var("APP_ENV").as_deref() == Ok("dev") { "debug".into() } else { "warn".into() }
        }))
        .with(tracing_subscriber::fmt::layer().with_ansi(true).with_target(true))
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false).with_target(true))
        .init();
    // Prevenim drop-ul guard-ului
    let _ = _guard;
}

// ============================================================================
// DB bootstrap
// ============================================================================

pub(crate) async fn init_db(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS products (
        id SERIAL PRIMARY KEY, brand TEXT NOT NULL, name TEXT NOT NULL,
        slug TEXT UNIQUE NOT NULL, category_id INTEGER NOT NULL DEFAULT 1,
        release_year INTEGER, specs JSONB NOT NULL DEFAULT '{}',
        price_new INTEGER, affiliate_url TEXT, image_url TEXT,
        created_at TIMESTAMPTZ DEFAULT NOW()
    )").execute(pool).await?;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_products_slug ON products(slug)").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE products ADD COLUMN IF NOT EXISTS stock_count INTEGER NOT NULL DEFAULT 0").execute(pool).await;
    let _ = sqlx::query("UPDATE products SET stock_count = 10 WHERE stock_count IS NULL OR stock_count = 0").execute(pool).await;

    // Tabela categorii (flată — compatibilă cu PgProductRepo::get_categories)
    let _ = sqlx::query(r#"CREATE TABLE IF NOT EXISTS categories (
        id SERIAL PRIMARY KEY, name TEXT NOT NULL, slug TEXT UNIQUE NOT NULL
    )"#).execute(pool).await;
    let cat_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories").fetch_one(pool).await.unwrap_or(0);
    if cat_count == 0 {
        sqlx::query("INSERT INTO categories (name, slug) VALUES
            ('Telefoane', 'telefoane'), ('Tablete', 'tablete'),
            ('Laptopuri', 'laptopuri'), ('Audio', 'audio'), ('Accesorii', 'accesorii')
        ON CONFLICT DO NOTHING").execute(pool).await.ok();
    }
    tracing::info!("✅ Baza de date inițializată");
    Ok(())
}

// ============================================================================
// Tera setup
// ============================================================================

fn init_tera() -> tera::Tera {
    let mut tera = Tera::new();
    for (pattern, dir) in &[
        ("shop-mvp/templates/**/*.html", "shop-mvp/templates"),
        ("templates/**/*.html", "templates"),
    ] {
        if std::path::Path::new(dir).exists() {
            if let Err(e) = tera.load_from_glob(pattern) {
                tracing::warn!("Tera load {}: {}", dir, e);
            }
        }
    }
    tera
}

// ============================================================================
// LEGO assembly — toate modulele
// ============================================================================

pub struct LegoModules {
    pub products: Arc<dyn ProductRepo>,
    pub auth: Arc<dyn rust_auth::AuthRepo>,
    pub cart: Arc<dyn rust_cart::CartRepo>,
    pub orders: Arc<dyn rust_marketplace_orders::OrderRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub renderer: RenderService,
    pub site_url: String,
    pub max_qty: i32,
    pub stripe_webhook_secret: String,
    pub pool: PgPool,
}

pub async fn assemble_lego_modules(cfg: &AppConfig, pool: &PgPool) -> anyhow::Result<LegoModules> {
    // CategoryService real
    let categories_svc = rust_marketplace_categories::PgCategoryRepo::new(pool.clone());

    let products: Arc<dyn ProductRepo> = Arc::new(PgProductRepo::new(
        pool.clone(),
        Box::new(categories_svc) as Box<dyn rust_marketplace_products::CategoryService>,
    ));
    tracing::info!("🧱 Produse asamblat");

    let auth: Arc<dyn rust_auth::AuthRepo> = Arc::new(PgAuthRepo::new(pool.clone(), &cfg.jwt_secret));
    auth.migrate().await?;
    tracing::info!("🧱 Autentificare asamblat");

    let cart: Arc<dyn rust_cart::CartRepo> = Arc::new(PgCartRepo::new(pool.clone()));
    cart.migrate().await?;
    tracing::info!("🧱 Coș asamblat");

    let orders: Arc<dyn rust_marketplace_orders::OrderRepo> = Arc::new(PgOrderRepo::new(pool.clone()));
    orders.migrate().await?;
    orders.migrate_idempotency().await?;
    tracing::info!("🧱 Comenzi asamblat");

    let payment: Arc<dyn PaymentRepo> = if cfg.mock_payment {
        tracing::warn!("🔧 MOCK_PAYMENT=true — plată instant, fără Stripe!");
        Arc::new(MockPayment::new())
    } else {
        Arc::new(RetryPayment::new(
            Arc::new(StripePayment::new(&cfg.stripe_secret))
        ))
    };
    tracing::info!("💳 Payment asamblat");

    let renderer = RenderService::new(init_tera());

    Ok(LegoModules {
        products, auth, cart, orders, payment,
        renderer,
        site_url: cfg.site_url.clone(),
        max_qty: cfg.max_qty,
        stripe_webhook_secret: cfg.stripe_webhook_secret.clone(),
        pool: pool.clone(),
    })
}

// ============================================================================
// Build app — asamblează state + router + server
// ============================================================================

pub async fn build_and_serve(lego: LegoModules) -> anyhow::Result<()> {
    let inner_router = boundary::build_inner_router(
        &lego.products, &lego.auth, &lego.cart, &lego.orders, &lego.payment,
        &lego.renderer, &lego.site_url, lego.max_qty, &lego.stripe_webhook_secret, &lego.pool,
    );

    let state = AppState {
        products: lego.products,
        auth: lego.auth,
        cart: lego.cart,
        orders: lego.orders,
        payment: lego.payment,
        renderer: lego.renderer,
        site_url: lego.site_url,
        max_qty: lego.max_qty,
        stripe_webhook_secret: lego.stripe_webhook_secret,
        db: lego.pool,
        fc: FcState { inner_router: Arc::new(inner_router) },
    };

    let app = boundary::build_outer_router(state);

    let addr = format!("0.0.0.0:{}", std::env::var("PORT").unwrap_or_else(|_| "3001".into()));
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "prod".to_string());
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "default".to_string());

    tracing::info!("🚀 Shop-MVP pornit pe http://{addr}");
    tracing::info!("📋 Config: env={app_env} db={log_level} port={}", &addr[addr.rfind(':').map(|i| i+1).unwrap_or(0)..]);
    tracing::info!("📋 Rute active:");
    for route in &[
        "GET/POST /", "GET /products", "GET /product/{slug}", "GET /search",
        "GET/POST /login", "GET/POST /signup", "GET/POST /logout", "GET /me",
        "GET /cart", "POST /cart/add", "POST /cart/remove", "POST /cart/update",
        "GET /checkout", "POST /checkout",
        "GET /orders", "POST /order/{id}/pay",
        "GET /success",
        "POST /stripe/webhook",
        "GET /admin", "GET /admin/orders",
        "POST /admin/order/{id}/status",
        "GET/POST /admin/product/new", "GET/POST /admin/product/{slug}/edit",
        "POST /admin/product/{slug}/delete",
        "GET /admin/logs", "POST /admin/migrate-orders",
        "POST /account/delete", "GET /account/export",
        "GET /privacy", "GET /security",
        "GET /health",
        "GET /.well-known/security.txt",
    ] {
        tracing::info!("   📍 {}", route);
    }

    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        tracing::error!("❌ Nu pot porni pe portul {}: {e}", &addr);
        anyhow::anyhow!("Failed to bind to {addr}: {e}")
    })?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("👋 Server oprit gracefully");
    Ok(())
}

// ============================================================================
// Graceful shutdown
// ============================================================================

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.unwrap_or_else(|e| {
            tracing::error!("ctrl-c error: {e}");
        });
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .unwrap_or_else(|e| {
                tracing::error!("signal handler error: {e}");
                panic!("signal: {e}");
            })
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::warn!("📴 Semnal de oprire primit, se închid conexiunile...");
}

// ============================================================================
// DB query counter (folosit de admin/logs)
// ============================================================================

static DB_QUERY_COUNT: AtomicU64 = AtomicU64::new(0);
static DB_QUERY_LOG: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

pub fn reset_query_count() { DB_QUERY_COUNT.store(0, Ordering::Relaxed); }
pub fn get_query_count() -> u64 { DB_QUERY_COUNT.load(Ordering::Relaxed) }

pub fn count_query(sql: &str) {
    DB_QUERY_COUNT.fetch_add(1, Ordering::Relaxed);
    let n = get_query_count();
    if let Ok(mut log) = DB_QUERY_LOG.lock() {
        log.push(format!("#{} — {}", n, sql));
        if log.len() > 1000 { log.remove(0); }
    }
    crate::debug_log!(target: "sql", "[#{}] {}", n, sql);
}

pub fn get_query_log() -> Vec<String> {
    DB_QUERY_LOG.lock().map(|l| l.clone()).unwrap_or_default()
}

// ============================================================================
// Account lockout (ASVS L2)
// ============================================================================

static LOCKOUT_CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, (usize, std::time::Instant)>>> = std::sync::OnceLock::new();

fn get_lockout_map() -> &'static std::sync::Mutex<std::collections::HashMap<String, (usize, std::time::Instant)>> {
    LOCKOUT_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

pub fn check_lockout(ip: &str, email: &str) -> Result<(), &'static str> {
    let key = format!("{}:{}", ip, email);
    let mut map = get_lockout_map().lock().expect("lockout Mutex poisoned");
    if let Some((count, until)) = map.get(&key) {
        if *count >= 5 {
            if std::time::Instant::now() < *until {
                return Err("Cont blocat temporar. Încearcă din nou peste 15 minute.");
            } else {
                map.remove(&key);
            }
        }
    }
    Ok(())
}

pub fn record_failed_attempt(ip: &str, email: &str) {
    let key = format!("{}:{}", ip, email);
    let mut map = get_lockout_map().lock().expect("lockout Mutex poisoned");
    let entry = map.entry(key).or_insert((0, std::time::Instant::now()));
    entry.0 += 1;
    if entry.0 >= 5 {
        entry.1 = std::time::Instant::now() + std::time::Duration::from_secs(15 * 60);
    }
}

pub fn clear_lockout(ip: &str, email: &str) {
    let key = format!("{}:{}", ip, email);
    let mut map = get_lockout_map().lock().expect("lockout Mutex poisoned");
    map.remove(&key);
}

// ============================================================================
// Constants
// ============================================================================

pub const MAX_ITEMS_PER_ORDER: usize = 20;
pub const MAX_ORDER_VALUE_BANI: i64 = 10_000_00; // 10,000 lei
