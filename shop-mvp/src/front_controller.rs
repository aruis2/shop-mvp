// =============================================================================
// 🚦 Front Controller V3 — UNICUL punct de intrare/ieșire (TrustBoundary complet)
// =============================================================================
// FILOSOFIE: TRUST-BOUNDARY.md — tot ce intră/iese trece PRIN AICI
// STANDARD: OWASP ASVS V5.1 (Input Validation), V10 (Output Encoding)
//            OWASP API Top 10 #1 (BOLA)
// =============================================================================
//
// Acest modul înlocuiește toate rutele împrăștiate din main.rs.
// Rutele sunt DEFINITE aici, centralizat.
//
// Middleware:
//   TrustBoundary (validare input) + SafeResponse (headere securitate automate)
//   = înlocuiește csrf_middleware + security_headers + session_timeout
// =============================================================================

use axum::{
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use rust_url_normalizer::strip_trailing_slash;
use tower_http::services::ServeDir;

use crate::handlers::{admin, auth, cart, orders, products};
use crate::state::AppState;
use crate::trust_boundary;

/// Construiește UNICA rută a aplicației.
///
/// Toate rutele sunt declarate AICI, nu în main.rs.
/// TrustBoundary middleware rulează înaintea ORICĂRUI handler.
pub fn build_router(state: AppState) -> Router {
    let app = Router::new()
        // ─── Home & Produse ──────────────────
        .route("/", get(products::home_page))
        .route("/products", get(products::products_page))
        .route("/product/{slug}", get(products::product_detail_page))
        .route("/search", get(products::search_page))

        // ─── Coș ──────────────────────────────
        .route("/cart", get(cart::cart_page))
        .route("/cart/add", post(cart::cart_add))
        .route("/cart/remove", post(cart::cart_remove))
        .route("/cart/update", post(cart::cart_update))

        // ─── Autentificare ────────────────────
        .route("/login", get(auth::login_page).post(auth::login_handler))
        .route("/signup", get(auth::signup_page).post(auth::signup_handler))
        .route("/logout", get(auth::logout_handler).post(auth::logout_handler))
        .route("/me", get(auth::me_handler))

        // ─── Checkout & Comenzi ───────────────
        .route("/checkout", get(orders::checkout_page).post(orders::checkout_handler))
        .route("/order/{id}/pay", post(orders::order_pay))
        .route("/orders", get(orders::orders_page))
        .route("/success", get(orders::success_page))
        .route("/stripe/webhook", post(orders::stripe_webhook))

        // ─── Admin ────────────────────────────
        .route("/admin", get(admin::admin_products_page))
        .route("/admin/orders", get(admin::admin_orders_page))
        .route("/admin/order/{id}/status", post(admin::admin_order_update_status))
        .route("/admin/product/new", get(admin::admin_product_new_page).post(admin::admin_product_create))
        .route("/admin/product/{slug}/edit", get(admin::admin_product_edit_page).post(admin::admin_product_update))
        .route("/admin/product/{slug}/delete", post(admin::admin_product_delete))
        .route("/admin/logs", get(admin::admin_logs))
        .route("/admin/migrate-orders", post(admin::admin_migrate_orders))

        // ─── GDPR & Informațional ────────────
        .route("/account/delete", post(auth::delete_account_handler))
        .route("/account/export", get(auth::export_data_handler))
        .route("/privacy", get(auth::privacy_policy_page))
        .route("/security", get(auth::security_policy_page))

        // ─── Utile ───────────────────────────
        .route("/health", get(health_check))
        .route("/.well-known/security.txt", get(security_txt))

        // ─── Fișiere statice ─────────────────
        .nest_service("/static", ServeDir::new("shop-mvp/static"))

        // ─── Middleware ──────────────────────
        // 🔐 TrustBoundary — CSRF, path/header/cookie validation, logging
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            trust_boundary::trust_boundary_middleware,
        ))
        // 🔗 URL normalization — trailing slash redirect 301
        .layer(middleware::from_fn(strip_trailing_slash))
        // 📦 Body size limit
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))

        // ─── State ───────────────────────────
        .with_state(state);

    app
}

// ============================================================================
// Handlere interne (rămân aici până la migrarea completă la SafeResponse)
// ============================================================================

async fn health_check() -> impl axum::response::IntoResponse {
    (axum::http::StatusCode::OK, "OK")
}

async fn security_txt() -> impl IntoResponse {
    let contact = std::env::var("SECURITY_CONTACT")
        .unwrap_or_else(|_| "security@example.com".into());
    let content = format!(
        "Contact: mailto:{}\nPreferred-Languages: ro, en\n",
        contact
    );
    (axum::http::StatusCode::OK, [("Content-Type", "text/plain")], content)
}
