// =============================================================================
// 🛒 shop-mvp — Bootstrap (arhitectură capability-based, seL4-style)
// =============================================================================
// Handlerele sunt în module separate, fiecare cu propriul domain state.
// Niciun handler nu primește AppState direct — primește doar capabilitățile
// de care are nevoie (AuthState, ProductState, CartState, OrderState, AdminState).

// 🔧 Debug mode: suprimăm warning-urile de dead code (normal pentru alpha)
// Acestea vor fi eliminate la release când toate componentele sunt în uz.
#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use sqlx::PgPool;
use tera::Tera;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use rust_marketplace_products::{PgProductRepo, ProductRepo};
use rust_auth::PgAuthRepo;
use rust_cart::PgCartRepo;
use rust_marketplace_orders::PgOrderRepo;
use rust_payment::{PaymentRepo, StripePayment, MockPayment};

mod url_encode;

mod cookie;
mod debug;
mod front_controller;
mod handlers;
mod trust_boundary;
mod payment_retry;
mod ratelimit;
mod render;
mod state;
mod types;

use payment_retry::RetryPayment;
use render::RenderService;
use state::*;

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // --- Panic hook: prinde crash-urile în log înainte să dispară ---
    let hook_panic_log = std::path::PathBuf::from("logs/panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        let location = info.location().map(|l| l.to_string()).unwrap_or_default();
        let backtrace = std::backtrace::Backtrace::capture();
        let panic_log = format!(
            "=== PANIC ===\n{}\nLocation: {}\nBacktrace:\n{:?}\n==============",
            msg, location, backtrace
        );
        // Scrie în fișierul de panică
        let _ = std::fs::OpenOptions::new()
            .create(true).append(true).open(&hook_panic_log)
            .and_then(|mut f| std::io::Write::write_all(&mut f, format!("{}\n", panic_log).as_bytes()));
        // Și în stderr (default)
        eprintln!("{}", panic_log);
    }));

    // --- Logging ---
    tokio::fs::create_dir_all("logs").await.ok();
    let file_appender = tracing_appender::rolling::daily("logs", "shop-mvp.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            if std::env::var("APP_ENV").as_deref() == Ok("dev") { "debug".into() } else { "warn".into() }
        }))
        .with(tracing_subscriber::fmt::layer().with_ansi(true).with_target(true))
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false).with_target(true))
        .init();

    // --- Config ---
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:123123@localhost:5432/test".into());
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set in environment — generează un string random: openssl rand -hex 32");
    let stripe_secret = std::env::var("STRIPE_SECRET_KEY")
        .expect("STRIPE_SECRET_KEY must be set in environment");
    let site_url = std::env::var("SITE_URL")
        .unwrap_or_else(|_| "http://localhost:3001".into())
        .trim_end_matches('/')
        .to_string();
    let max_qty: i32 = std::env::var("MAX_QTY_PER_PRODUCT")
        .unwrap_or_else(|_| "999".into())
        .parse()
        .unwrap_or(999);

    // --- DB ---
    let pool = PgPool::connect(&database_url).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS products (
        id SERIAL PRIMARY KEY, brand TEXT NOT NULL, name TEXT NOT NULL,
        slug TEXT UNIQUE NOT NULL, category_id INTEGER NOT NULL DEFAULT 1,
        release_year INTEGER, specs JSONB NOT NULL DEFAULT '{}',
        price_new INTEGER, affiliate_url TEXT, image_url TEXT,
        created_at TIMESTAMPTZ DEFAULT NOW()
    )").execute(&pool).await?;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_products_slug ON products(slug)").execute(&pool).await;
    // Coloana stock_count (adaug doar dacă nu există)
    let _ = sqlx::query("ALTER TABLE products ADD COLUMN IF NOT EXISTS stock_count INTEGER NOT NULL DEFAULT 0").execute(&pool).await;
    // Setează stock_count default la 10 pentru produsele existente
    let _ = sqlx::query("UPDATE products SET stock_count = 10 WHERE stock_count IS NULL OR stock_count = 0").execute(&pool).await;
    // Tabela categorii
    let _ = sqlx::query(r#"CREATE TABLE IF NOT EXISTS categories (
        id SERIAL PRIMARY KEY, name TEXT NOT NULL, slug TEXT UNIQUE NOT NULL
    )"#).execute(&pool).await;
    // Populează categorii default dacă e goală
    let cat_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories").fetch_one(&pool).await.unwrap_or(0);
    if cat_count == 0 {
        sqlx::query("INSERT INTO categories (name, slug) VALUES ('Telefoane', 'telefoane'), ('Tablete', 'tablete'), ('Laptopuri', 'laptopuri'), ('Audio', 'audio'), ('Accesorii', 'accesorii') ON CONFLICT DO NOTHING").execute(&pool).await.ok();
    }
    tracing::info!("✅ Baza de date inițializată");

    // --- Dummy CategoryService (MVP) ---
    struct DummyCatSvc;
    #[async_trait::async_trait]
    impl rust_marketplace_products::CategoryService for DummyCatSvc {
        async fn category_exists(&self, _id: i32) -> Result<bool, rust_marketplace_products::ProductError> {
            Ok(true)
        }
    }

    // --- LEGO-uri 🧱 (toate prin trait-uri) ---
    let products: Arc<dyn ProductRepo> = Arc::new(PgProductRepo::new(pool.clone(), Box::new(DummyCatSvc)));
    tracing::info!("🧱 Produse asamblat");

    let auth: Arc<dyn rust_auth::AuthRepo> = Arc::new(PgAuthRepo::new(pool.clone(), &jwt_secret));
    auth.migrate().await?;
    tracing::info!("🧱 Autentificare asamblat");

    let cart: Arc<dyn rust_cart::CartRepo> = Arc::new(PgCartRepo::new(pool.clone()));
    cart.migrate().await?;
    tracing::info!("🧱 Coș asamblat");

    let orders: Arc<dyn rust_marketplace_orders::OrderRepo> = Arc::new(PgOrderRepo::new(pool.clone()));
    orders.migrate().await?;
    orders.migrate_idempotency().await?;
    tracing::info!("🧱 Comenzi asamblat");

    // Stripe cu error boundary (retry + timeout)
    // 🔧 Dacă MOCK_PAYMENT=true, folosește plată instant (fără Stripe)
    let mock_payment = std::env::var("MOCK_PAYMENT").ok();
    let payment: Arc<dyn PaymentRepo> = if mock_payment.as_deref() == Some("true") {
        tracing::warn!("🔧 MOCK_PAYMENT=true — plată instant, fără Stripe!");
        Arc::new(MockPayment::new())
    } else {
        Arc::new(RetryPayment::new(
            Arc::new(StripePayment::new(&stripe_secret))
        ))
    };
    tracing::info!("💳 Payment asamblat");

    // --- RenderService (singurul punct de contact cu Tera) ---
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
    let renderer = RenderService::new(tera);

    // --- Master state (doar bootstrap, NU expus handlerelor) ---
    let state = AppState {
        products, auth, cart, orders, payment,
        renderer, site_url, max_qty, db: pool,
    };

    // ====================================================================
    // 🧩 Sub-rutere cu DOMAIN STATE-uri separate (capability-based)
    // ====================================================================
    // Fiecare sub-router are UN SINGUR tip de state și ACCES doar la
    // capabilitățile de care are nevoie. Imposibil ca un handler auth să
    // acceseze produse sau plăți — verificat la compilare.
    // ====================================================================

    // � Front Controller: UNICUL punct de intrare — rute definite centralizat
    // TrustBoundary middleware (CSRF, security headers, path/header/cookie validation)
    // e deja inclus în build_router()
    let app = front_controller::build_router(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".into());
    let addr = format!("0.0.0.0:{port}");
    let db_host = database_url.split('@').nth(1).unwrap_or("unknown").split('/').next().unwrap_or("unknown");
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "prod".to_string());
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "default".to_string());
    tracing::info!("🚀 Shop-MVP pornit pe http://{addr}");
    tracing::info!("📋 Config: env={app_env} db={db_host} log={log_level} port={port}");
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
        "GET /health",
        "GET /.well-known/security.txt",
    ] {
        tracing::info!("   📍 {}", route);
    }
    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        tracing::error!("❌ Nu pot porni pe portul {port}: {e}");
        anyhow::anyhow!("Failed to bind to {addr}: {e}")
    })?;

    // Graceful shutdown: SIGTERM/SIGINT → închide conexiunile
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("👋 Server oprit gracefully");
    Ok(())
}

// ============================================================================
// DB query counter
// ============================================================================

static DB_QUERY_COUNT: AtomicU64 = AtomicU64::new(0);
static DB_QUERY_LOG: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

pub fn reset_query_count() { DB_QUERY_COUNT.store(0, Ordering::Relaxed); }
pub fn get_query_count() -> u64 { DB_QUERY_COUNT.load(Ordering::Relaxed) }

/// Countează un query SQL pentru afișarea în /admin/logs
/// (sqlx face deja logare cu timing prin tracing, asta e doar pentru UI)
pub fn count_query(sql: &str) {
    DB_QUERY_COUNT.fetch_add(1, Ordering::Relaxed);
    let n = get_query_count();
    if let Ok(mut log) = DB_QUERY_LOG.lock() {
        log.push(format!("#{} — {}", n, sql));
        if log.len() > 1000 { log.remove(0); }
    }
    debug_log!(target: "sql", "[#{}] {}", n, sql);
}

pub fn get_query_log() -> Vec<String> {
    DB_QUERY_LOG.lock().map(|l| l.clone()).unwrap_or_default()
}

// 🔒 ASVS L2: Idempotency — mutată în DB (rust-marketplace-orders),
//              nu mai e nevoie de cache in-memory.
// ============================================================================
// Funcțiile anterioare (check_idempotency, store_idempotency_result) au fost
// înlocuite cu OrderRepo::check_idempotency() / store_idempotency()
// care folosesc tabela `idempotency_cache` în PostgreSQL.
// Beneficiu: cache-ul supraviețuiește restarturilor serverului.

// 🔒 ASVS L2: Account lockout middleware
// ============================================================================

/// Urmărește încercările eșuate de login și blochează temporar contul.
/// Cheia de lockout e `ip:email` — previne DoS pe email (atacatorul nu poate bloca un cont
/// doar trimițînd requesturi din IP-uri diferite, dar nici dintr-un singur IP nu poate
/// bloca un cont — fiecare IP are propriul contor).
static LOCKOUT_CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, (usize, std::time::Instant)>>> = std::sync::OnceLock::new();

fn get_lockout_map() -> &'static std::sync::Mutex<std::collections::HashMap<String, (usize, std::time::Instant)>> {
    LOCKOUT_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Verifică dacă un email de la un IP e blocat temporar (după 5 încercări eșuate, 15min lockout)
fn check_lockout(ip: &str, email: &str) -> Result<(), &'static str> {
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

/// Înregistrează o încercare eșuată de login (per IP:email)
fn record_failed_attempt(ip: &str, email: &str) {
    let key = format!("{}:{}", ip, email);
    let mut map = get_lockout_map().lock().expect("lockout Mutex poisoned");
    let entry = map.entry(key).or_insert((0, std::time::Instant::now()));
    entry.0 += 1;
    if entry.0 >= 5 {
        entry.1 = std::time::Instant::now() + std::time::Duration::from_secs(15 * 60); // 15min lockout
    }
}

/// Resetează contorul după login reușit (per IP:email)
fn clear_lockout(ip: &str, email: &str) {
    let key = format!("{}:{}", ip, email);
    let mut map = get_lockout_map().lock().expect("lockout Mutex poisoned");
    map.remove(&key);
}

// ============================================================================
// 🔒 ASVS L2: Business logic limits
// ============================================================================

const MAX_ITEMS_PER_ORDER: usize = 20;
const MAX_ORDER_VALUE_BANI: i64 = 10_000_00; // 10,000 lei

// ============================================================================
// Middleware legacy — înlocuit de TrustBoundary (trust_boundary.rs)
// Security headers, CSRF, session timeout, request timing sunt acum în
// front_controller.rs, gestionate de TrustBoundary middleware.
// ============================================================================

// ============================================================================
// Graceful shutdown signal
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
// Middleware — request timing + query count
// ============================================================================

async fn request_timing(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let req_id = uuid::Uuid::new_v4().to_string();

    reset_query_count();
    let start = std::time::Instant::now();

    // Span activ pe durata request-ului — toate logurile din interior îl moștenesc
    let span = tracing::info_span!("req", id = %req_id, method = %method, path = %path);
    let _guard = span.enter();

    let resp = next.run(req).await;
    let elapsed = start.elapsed();
    let status = resp.status();
    let qty = get_query_count();
    let code = status.as_u16();

    // Logare diferențiată după status code
    if code >= 500 {
        tracing::error!(target: "http", "[{req_id}] {method} {path}{query} → {code}  {elapsed:?}  [{qty} query-uri]  ⚠️ SERVER ERROR");
    } else if code >= 400 {
        tracing::warn!(target: "http",  "[{req_id}] {method} {path}{query} → {code}  {elapsed:?}  [{qty} query-uri]  ⚠️ CLIENT ERROR");
    } else if code >= 300 {
        let location = resp.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or("");
        debug_log!(target: "http", "[{req_id}] {method} {path}{query} → {code} -> {location}  {elapsed:?}  [{qty} query-uri]");
    } else {
        debug_log!(target: "http", "[{req_id}] {method} {path}{query} → {code}  {elapsed:?}  [{qty} query-uri]");
    }

    // Adaugă request_id în headerul răspunsului (ajută la debugging client-side)
    let (mut parts, body) = resp.into_parts();
    parts.headers.insert(
        axum::http::header::HeaderName::from_static("x-request-id"),
        axum::http::HeaderValue::from_str(&req_id).unwrap(),
    );
    axum::response::Response::from_parts(parts, body)
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tera::{Context, Tera};

    fn setup_tera() -> Tera {
        let mut tera = Tera::new();
        tera.autoescape_on(vec![".html", ".xml"]);
        for (pattern, dir) in &[
            ("shop-mvp/templates/**/*.html", "shop-mvp/templates"),
            ("templates/**/*.html", "templates"),
        ] {
            if std::path::Path::new(dir).exists() {
                tera.load_from_glob(pattern).expect("Tera load");
            }
        }
        tera
    }

    fn render_template(tera: &Tera, name: &str, ctx: &Context) {
        let result = tera.render(name, ctx);
        assert!(result.is_ok(), "Template '{name}': {:#?}", result.err());
    }

    #[test]
    fn test_index() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        render_template(&tera, "index.html", &ctx);
    }

    #[test]
    fn test_login() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        render_template(&tera, "auth/login.html", &ctx);
    }

    #[test]
    fn test_signup() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        render_template(&tera, "auth/signup.html", &ctx);
    }

    #[test]
    fn test_products() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("products", &json!([
            {"name":"Test","brand":"B","slug":"t","price_lei":"99.99","image_url":null,"stock_count":10}
        ]));
        ctx.insert("total", &1i64);
        ctx.insert("page", &1i64);
        ctx.insert("total_pages", &1i64);
        ctx.insert("categories", &json!([]));
        ctx.insert("category_id", &serde_json::Value::Null);
        render_template(&tera, "products/products.html", &ctx);
    }

    #[test]
    fn test_search() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("query", "test");
        ctx.insert("products", &json!([
            {"name":"Test","brand":"B","slug":"t","price_lei":"99.99","image_url":null}
        ]));
        ctx.insert("total", &1i64);
        ctx.insert("page", &1i64);
        ctx.insert("total_pages", &1i64);
        render_template(&tera, "products/search.html", &ctx);
    }

    #[test]
    fn test_product_detail() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("product", &json!({
            "name":"Test","brand":"B","slug":"test","price_lei":"99.99",
            "image_url":null,
            "specs":[{"key":"Culoare","value":"Albastru"}]
        }));
        render_template(&tera, "products/product_detail.html", &ctx);
    }

    #[test]
    fn test_cart() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("cart_items", &json!([
            {"product_name":"Test","price_lei":"99.99","current_price_lei":"99.99",
             "qty":2,"subtotal_lei":"199.98","id":"uuid"}
        ]));
        ctx.insert("total_lei", "199.98");
        render_template(&tera, "cart/cart.html", &ctx);
    }

    #[test]
    fn test_checkout() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("total_lei", "199.98");
        ctx.insert("session_id", "sess_123");
        ctx.insert("item_count", &2i64);
        render_template(&tera, "orders/checkout.html", &ctx);
    }

    #[test]
    fn test_orders() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("orders", &json!([
            {"id":"ord-1","created_at":"2026-07-09","status":"pending",
             "payment_status":"unpaid","total_lei":"199.98",
             "shipping_name":"Ion","shipping_address":"Str. X"}
        ]));
        ctx.insert("total_pages", &1i64);
        ctx.insert("page", &1i64);
        render_template(&tera, "orders/orders.html", &ctx);
    }

    #[test]
    fn test_success() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        render_template(&tera, "orders/success.html", &ctx);
    }

    #[test]
    fn test_admin_products() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("products", &json!([
            {"brand":"B","name":"Test","slug":"t","price_lei":"99.99","stock_count":10}
        ]));
        ctx.insert("total", &1i64);
        ctx.insert("page", &1i64);
        ctx.insert("total_pages", &1i64);
        render_template(&tera, "admin/admin_products.html", &ctx);
    }

    #[test]
    fn test_admin_product_form_new() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("product", &serde_json::Value::Null);
        render_template(&tera, "admin/admin_product_form.html", &ctx);
    }

    #[test]
    fn test_admin_product_form_edit() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("product", &json!({
            "brand":"B","name":"Test","slug":"t","price_new":9999,"price_lei":"99.99"
        }));
        render_template(&tera, "admin/admin_product_form.html", &ctx);
    }

    #[test]
    fn test_admin_orders() {
        let tera = setup_tera();
        let mut ctx = Context::new();
        ctx.insert("base_path", "");
        ctx.insert("orders", &json!([
            {"id":"o1","created_at":"2026-07-09","status":"pending",
             "payment_status":"unpaid","total_lei":"199.98",
             "shipping_name":"Ion","shipping_address":"Str. X","shipping_phone":"0722..."}
        ]));
        ctx.insert("total", &1i64);
        ctx.insert("page", &1i64);
        ctx.insert("total_pages", &1i64);
        render_template(&tera, "admin/admin_orders.html", &ctx);
    }
}

#[cfg(test)]
mod db_tests {
    use sqlx::{PgPool, Row};

    async fn get_pool() -> PgPool {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:123123@localhost:5432/test".into());
        PgPool::connect(&url).await.expect("DB connect")
    }

    #[tokio::test]
    async fn test_products_has_all_columns() {
        let pool = get_pool().await;
        let row = sqlx::query("SELECT column_name, data_type FROM information_schema.columns WHERE table_name='products'")
            .fetch_all(&pool).await.expect("query");
        let cols: Vec<String> = row.iter().map(|r| r.get::<String, _>("column_name")).collect();
        for c in &["id","brand","name","slug","category_id","specs","price_new","created_at"] {
            assert!(cols.contains(&c.to_string()), "Missing column: {c}");
        }
    }

    #[tokio::test]
    async fn test_cart_items_has_all_columns() {
        let pool = get_pool().await;
        let row = sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_name='cart_items'")
            .fetch_all(&pool).await.expect("query");
        let cols: Vec<String> = row.iter().map(|r| r.get::<String, _>("column_name")).collect();
        for c in &["id","session_id","product_slug","price_bani","qty"] {
            assert!(cols.contains(&c.to_string()), "Missing column: {c}");
        }
    }

    #[tokio::test]
    async fn test_orders_has_payment_columns() {
        let pool = get_pool().await;
        let row = sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_name='orders'")
            .fetch_all(&pool).await.expect("query");
        let cols: Vec<String> = row.iter().map(|r| r.get::<String, _>("column_name")).collect();
        for c in &["id","status","payment_status","total_bani","payment_provider","payment_provider_id"] {
            assert!(cols.contains(&c.to_string()), "Missing column: {c}");
        }
    }

    #[tokio::test]
    async fn test_order_items_has_all_columns() {
        let pool = get_pool().await;
        let row = sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_name='order_items'")
            .fetch_all(&pool).await.expect("query");
        let cols: Vec<String> = row.iter().map(|r| r.get::<String, _>("column_name")).collect();
        for c in &["id","order_id","product_slug","price_bani","qty"] {
            assert!(cols.contains(&c.to_string()), "Missing column: {c}");
        }
    }

    #[tokio::test]
    async fn test_users_has_auth_columns() {
        let pool = get_pool().await;
        let row = sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_name='users'")
            .fetch_all(&pool).await.expect("query");
        let cols: Vec<String> = row.iter().map(|r| r.get::<String, _>("column_name")).collect();
        for c in &["id","email","password_hash","role"] {
            assert!(cols.contains(&c.to_string()), "Missing column: {c}");
        }
    }
}
