// =============================================================================
// 🛒 shop-mvp — Bootstrap (arhitectură capability-based, seL4-style)
// =============================================================================
// Handlerele sunt în module separate, fiecare cu propriul domain state.
// Niciun handler nu primește AppState direct — primește doar capabilitățile
// de care are nevoie (AuthState, ProductState, CartState, OrderState, AdminState).
//
// Logica de startup a fost extrasă în startup.rs — main.rs e doar punctul
// de intrare.
// =============================================================================

#![allow(dead_code)]

mod http;
mod boundary;
mod cookie;
mod debug;
mod front_controller;
mod handlers;
mod startup;
mod trust_boundary;
mod render;
mod state;
mod types;

use startup::*;

// ============================================================================
// Main — doar orchestrare
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_panic_hook();
    init_logging();
    tokio::fs::create_dir_all("logs").await.ok();

    let cfg = AppConfig::from_env();
    let pool = sqlx::PgPool::connect(&cfg.database_url).await?;
    startup::init_db(&pool).await?;

    let lego = startup::assemble_lego_modules(&cfg, &pool).await?;
    startup::build_and_serve(lego).await
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
