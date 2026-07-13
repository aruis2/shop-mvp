// =============================================================================
// 🔐 TrustBoundary Middleware — validatează ORICE request la graniță
// =============================================================================
// FILOSOFIE: TRUST-BOUNDARY.md — tot ce intră trece pe aici
// STANDARD: OWASP ASVS V5.1 (Input Validation), V10 (Output Encoding)
//            OWASP API Top 10 #1 (BOLA)
// =============================================================================
//
// Acest middleware rulează înaintea ORICĂRUI handler.
// Parsează și validează request-ul cu TrustBoundary, apoi stochează
// SafeRequest în extensiile request-ului.
//
// Handlerele pot accesa SafeRequest prin:
//   req.extensions().get::<SafeRequestExtension>().unwrap()
//
// Treptat, toate handlerele vor migra la SafeRequest și acest middleware
// va fi singurul punct de intrare.

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use rust_trust_boundary::{SafeMethod, SafePath, SafeHeaders, SafeCookies};
use crate::debug_warn;

/// Extension key pentru SafeRequest — handlerele îl extrag din request.
/// NOTĂ: Body-ul nu e validat aici (e consumat de Axum înainte de middleware).
///       Validarea body-ului rămâne la nivel de handler (InputFactory).
#[derive(Clone)]
pub struct SafeRequestPartial {
    pub method: SafeMethod,
    pub path: SafePath,
    pub headers: SafeHeaders,
    pub cookies: SafeCookies,
    pub client_ip: String,
    pub request_id: String,
}

/// Middleware care validează HEADERELE, COOKIE-URILE și PATH-UL la graniță.
///
/// Body-ul nu poate fi validat în middleware (e consumat de Axum).
/// Validarea body-ului rămâne la InputFactory, în handlere.
pub async fn trust_boundary_middleware(
    axum::extract::State(s): axum::extract::State<crate::state::AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let request_id = format!("{:x}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos());

    // 1. Validează path-ul
    let path = match SafePath::parse(req.uri().path()) {
        Ok(p) => p,
        Err(e) => {
            debug_warn!(target: "boundary", "Path invalid: {}", e);
            return (axum::http::StatusCode::BAD_REQUEST, "Path invalid").into_response();
        }
    };

    // 2. Validează headerele
    let headers = match SafeHeaders::parse(req.headers()) {
        Ok(h) => h,
        Err(e) => {
            debug_warn!(target: "boundary", "Headers invalid: {}", e);
            return (axum::http::StatusCode::BAD_REQUEST, "Headers invalid").into_response();
        }
    };

    // 3. Parsează cookie-urile
    let cookie_header = headers.get("cookie");
    let cookies = match SafeCookies::parse_from_header(cookie_header) {
        Ok(c) => c,
        Err(e) => {
            debug_warn!(target: "boundary", "Cookies invalid: {}", e);
            return (axum::http::StatusCode::BAD_REQUEST, "Cookie invalid").into_response();
        }
    };

    // 4. Metoda
    let method = match req.method().as_str() {
        "GET" => SafeMethod::Get,
        "POST" => SafeMethod::Post,
        _ => SafeMethod::Get,
    };

    // 5. IP client
    let client_ip = headers.client_ip().unwrap_or("unknown").to_string();

    // 6. Verificare CSRF (doar pentru POST)
    if method.is_post() && !headers.verify_csrf(&s.site_url) {
        debug_warn!(
            target: "csrf",
            "CSRF respins: {} {} origin={:?} referer={:?}",
            method_str(method), path, headers.origin(), headers.referer(),
        );
        return (axum::http::StatusCode::FORBIDDEN, "CSRF respins").into_response();
    }

    // Logare
    tracing::debug!(
        target: "boundary",
        "[{}] {} {} (IP: {})",
        request_id, method_str(method), path, client_ip,
    );

    // Stocare date validate în extensii
    let validated = SafeRequestPartial {
        method, path, headers, cookies, client_ip, request_id,
    };
    req.extensions_mut().insert(validated);

    next.run(req).await
}

fn method_str(m: rust_trust_boundary::SafeMethod) -> &'static str {
    match m {
        rust_trust_boundary::SafeMethod::Get => "GET",
        rust_trust_boundary::SafeMethod::Post => "POST",
        rust_trust_boundary::SafeMethod::Put => "PUT",
        rust_trust_boundary::SafeMethod::Delete => "DELETE",
        rust_trust_boundary::SafeMethod::Patch => "PATCH",
        rust_trust_boundary::SafeMethod::Head => "HEAD",
        rust_trust_boundary::SafeMethod::Options => "OPTIONS",
    }
}
