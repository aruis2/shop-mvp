// =============================================================================
// 🚦 Front Controller V5 — UNICUL punct de intrare/ieșire (fallback unic)
// =============================================================================
// FILOSOFIE: TRUST-BOUNDARY.md — tot ce intră/iese trece PRIN AICI
// STANDARD: OWASP ASVS V5.1 (Input Validation), V10 (Output Encoding)
//            OWASP API Top 10 #1 (BOLA)
// =============================================================================
//
// V5: O singură rută de fallback. TOATE request-urile trec prin:
//   1. TrustBoundary — validează TOT (path, headere, cookie, body, CSRF)
//   2. Inner router — reconstruiește request-ul și face routing normal
//
// NICIUN handler nu poate fi chemat fără să treacă prin TrustBoundary.
// =============================================================================

use std::sync::Arc;
use axum::{
    body::Body,
    extract::{FromRef, Request, State},
    http::HeaderValue,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use rust_trust_boundary::{SafeRequest, TrustBoundary};
use rust_url_normalizer::strip_trailing_slash;
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::handlers::{admin, auth, cart, orders, products};
use crate::state::{AppState, FcState};
use crate::trust_boundary as tb;

/// Construiește routerul INTERN — conține TOATE rutele reale.
pub fn build_inner_router(
    products: &Arc<dyn rust_marketplace_products::ProductRepo>,
    auth: &Arc<dyn rust_auth::AuthRepo>,
    cart: &Arc<dyn rust_cart::CartRepo>,
    orders: &Arc<dyn rust_marketplace_orders::OrderRepo>,
    payment: &Arc<dyn rust_payment::PaymentRepo>,
    renderer: &crate::render::RenderService,
    site_url: &str,
    max_qty: i32,
    db: &sqlx::PgPool,
) -> Router {
    let state = AppState {
        products: products.clone(),
        auth: auth.clone(),
        cart: cart.clone(),
        orders: orders.clone(),
        payment: payment.clone(),
        renderer: renderer.clone(),
        site_url: site_url.to_string(),
        max_qty,
        db: db.clone(),
        fc: FcState { inner_router: Arc::new(Router::new()) }, // temporary, replaced below
    };

    let router = Router::new()
        .route("/", get(products::home_page))
        .route("/products", get(products::products_page))
        .route("/product/{slug}", get(products::product_detail_page))
        .route("/search", get(products::search_page))
        .route("/cart", get(cart::cart_page))
        .route("/cart/add", post(cart::cart_add))
        .route("/cart/remove", post(cart::cart_remove))
        .route("/cart/update", post(cart::cart_update))
        .route("/login", get(auth::login_page).post(auth::login_handler))
        .route("/signup", get(auth::signup_page).post(auth::signup_handler))
        .route("/logout", get(auth::logout_handler).post(auth::logout_handler))
        .route("/me", get(auth::me_handler))
        .route("/checkout", get(orders::checkout_page).post(orders::checkout_handler))
        .route("/order/{id}/pay", post(orders::order_pay))
        .route("/orders", get(orders::orders_page))
        .route("/success", get(orders::success_page))
        .route("/stripe/webhook", post(orders::stripe_webhook))
        .route("/admin", get(admin::admin_products_page))
        .route("/admin/orders", get(admin::admin_orders_page))
        .route("/admin/order/{id}/status", post(admin::admin_order_update_status))
        .route("/admin/product/new", get(admin::admin_product_new_page).post(admin::admin_product_create))
        .route("/admin/product/{slug}/edit", get(admin::admin_product_edit_page).post(admin::admin_product_update))
        .route("/admin/product/{slug}/delete", post(admin::admin_product_delete))
        .route("/admin/logs", get(admin::admin_logs))
        .route("/admin/migrate-orders", post(admin::admin_migrate_orders))
        .route("/account/delete", post(auth::delete_account_handler))
        .route("/account/export", get(auth::export_data_handler))
        .route("/privacy", get(auth::privacy_policy_page))
        .route("/security", get(auth::security_policy_page))
        .route("/health", get(health_check))
        .route("/.well-known/security.txt", get(security_txt))
        .with_state(state);

    router
}

/// Construiește routerul EXTERN — o SINGURĂ rută de fallback.
pub fn build_outer_router(state: AppState) -> Router {
    Router::new()
        .fallback(fallback_handler)
        .nest_service("/static", ServeDir::new("shop-mvp/static"))
        // 🔐 TrustBoundary — validatează TOT la graniță
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            tb::trust_boundary_middleware,
        ))
        // 🔒 Security headers la ieșire
        .layer(middleware::from_fn(security_headers_middleware))
        // 🔗 URL normalization
        .layer(middleware::from_fn(strip_trailing_slash))
        // 📦 Body size limit
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state)
}

/// Handler unic de fallback — TOATE request-urile trec PRIN AICI.
async fn fallback_handler(
    State(state): State<AppState>,
    req: Request,
) -> Response {
    let fc: FcState = FcState::from_ref(&state);
    let request_id = SafeRequest::generate_request_id();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();
    let site_url = state.site_url.clone();

    // 1. Extrage body-ul ca bytes
    let body_bytes = match axum::body::to_bytes(req.into_body(), 2 * 1024 * 1024).await {
        Ok(b) => b.to_vec(),
        Err(_) => {
            return (axum::http::StatusCode::BAD_REQUEST, "Body prea mare").into_response();
        }
    };

    // 2. TrustBoundary validează TOT (path, headere, cookie, body, CSRF)
    match TrustBoundary::parse_parts_with_config(&method, &uri, &headers, &body_bytes, &site_url) {
        Ok(safe) => {
            // Logare
            tracing::info!(
                target: "http",
                "[{request_id}] {} {} (IP: {})",
                safe.method_str(), safe.path_str(), safe.client_ip,
            );

            // CSRF (doar POST)
            if safe.method.is_post() && !safe.verify_csrf() {
                tracing::warn!(target: "csrf", "CSRF respins: {} {}", safe.method_str(), safe.path_str());
                return (axum::http::StatusCode::FORBIDDEN, "CSRF respins").into_response();
            }

            // 3. Reconstruiește request-ul (metoda, uri, headere, body)
            let mut builder = http::Request::builder()
                .method(&method)
                .uri(&uri);
            for (name, value) in headers.iter() {
                builder = builder.header(name, value);
            }
            let new_req = builder
                .body(Body::from(body_bytes))
                .unwrap_or_else(|_| {
                    http::Request::new(Body::from(""))
                });

            // 4. Trimite la inner_router (rutele reale)
            match fc.inner_router.as_ref().clone().oneshot(new_req).await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::error!(target: "http", "Inner router error: {}", e);
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response()
                }
            }
        }
        Err(e) => {
            tracing::warn!(target: "boundary", "Request invalid: {}", e);
            let safe_resp = TrustBoundary::error_response(&e);
            // Convert SafeResponse to Axum Response
            let http_resp = safe_resp.into_http_response();
            let (parts, body_str) = http_resp.into_parts();
            Response::from_parts(parts, Body::from(body_str))
        }
    }
}

// ============================================================================
// Security headers middleware — adăugate AUTOMAT la ORICE răspuns
// ============================================================================

pub async fn security_headers_middleware(
    req: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let is_sensitive = req.uri().path().starts_with("/login")
        || req.uri().path().starts_with("/signup")
        || req.uri().path().starts_with("/checkout")
        || req.uri().path().starts_with("/admin")
        || req.uri().path().starts_with("/orders")
        || req.uri().path().starts_with("/me")
        || req.uri().path().starts_with("/cart");

    let resp = next.run(req).await;
    let (mut parts, body) = resp.into_parts();

    // ─── SANITIZARE BODY la granița de ieșire ─────────────────
    // V6: colectăm body-ul, îl sanitizăm cu OutputFactory, și-l punem înapoi.
    let body_bytes = match axum::body::to_bytes(body, 2 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            tracing::error!(target: "output", "Body prea mare la ieșire");
            return Response::from_parts(parts, Body::from(""));
        }
    };

    let content_type = parts
        .headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let sanitized_body: Body = if content_type.contains("text/plain") {
        // Sanitizare cu OutputFactory pentru text simplu (erori, mesaje)
        // HTML-ul de la Tera e deja safe (tera escapează automat) — nu dublăm encodingul
        let text = String::from_utf8_lossy(&body_bytes);
        let safe = crate::types::output::OutputFactory::text_html(&text);
        Body::from(safe.into_bytes())
    } else if content_type.contains("application/json") {
        // JSON: verificăm că e valid
        let text = String::from_utf8_lossy(&body_bytes);
        if serde_json::from_str::<serde_json::Value>(&text).is_err() {
            tracing::warn!(target: "output", "JSON invalid la ieșire");
            Body::from("{\"error\":\"internal error\"}")
        } else {
            Body::from(body_bytes.to_vec())
        }
    } else {
        // Alte tipuri (CSS, imagini, etc.) — nemodificate
        Body::from(body_bytes.to_vec())
    };

    // 🔒 Eliminăm Content-Length (se va recalcula automat de hyper)
    parts.headers.remove(axum::http::header::CONTENT_LENGTH);

    // ─── HEADERE DE SECURITATE ───────────────────────────────
    parts.headers.insert(
        axum::http::header::HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    if is_sensitive {
        parts.headers.insert(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
        );
    }
    let csp = match std::env::var("APP_ENV").unwrap_or_default().as_str() {
        "prod" | "production" => {
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; form-action 'self' https://checkout.stripe.com; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; upgrade-insecure-requests"
        }
        _ => {
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; form-action 'self' https://checkout.stripe.com; base-uri 'self'; frame-ancestors 'none'; object-src 'none'"
        }
    };
    parts.headers.insert(axum::http::header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(csp));
    parts.headers.insert(
        axum::http::header::HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    parts.headers.insert(
        axum::http::header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    parts.headers.insert(
        axum::http::header::HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    Response::from_parts(parts, sanitized_body)
}

// ============================================================================
// Handlere interne
// ============================================================================

async fn health_check() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "OK")
}

async fn security_txt() -> impl IntoResponse {
    let contact = std::env::var("SECURITY_CONTACT")
        .unwrap_or_else(|_| "security@example.com".into());
    let content = format!(
        "Contact: mailto:{}\nPreferred-Languages: ro, en\n", contact
    );
    (axum::http::StatusCode::OK, [("Content-Type", "text/plain")], content)
}
