// =============================================================================
// 🌐 HTTP Helpers — funcții generice pentru request/response
// =============================================================================
// Mutate din handlers pentru reutilizare. Zero dependențe de domeniu.
// =============================================================================

use axum::http::HeaderMap;
use crate::boundary::{SafeResponse, OutputFactory};

/// Extrage IP-ul clientului din X-Forwarded-For.
pub fn client_ip(headers: &HeaderMap) -> String {
    headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Extrage token-ul JWT din: Authorization header → cookie.
/// 🔒 Token-ul în query param e un risc de securitate.
pub fn extract_token<'a>(headers: &'a HeaderMap) -> Option<&'a str> {
    headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| {
            headers.get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|c| crate::cookie::get_cookie(c, "token"))
        })
}

/// 🔒 Redirect sigur — URL-ul trece prin OutputFactory::safe_redirect_url.
pub fn safe_redirect(dest: &str) -> SafeResponse {
    let safe = OutputFactory::safe_redirect_url(dest, "/")
        .unwrap_or_else(|| "/".to_string());
    SafeResponse::redirect(safe)
}

/// Redirect prin meta refresh (fără JS, CSP-safe).
pub fn redirect_html(url: &str) -> SafeResponse {
    let safe_url = OutputFactory::safe_redirect_url(url, "/")
        .unwrap_or_else(|| "/".to_string());
    SafeResponse::html(format!(
        r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0;url={safe_url}"></head><body><p><a href="{safe_url}">Continuă</a></p></body></html>"#
    ))
}

/// Redirect înapoi la referer cu mesaj de eroare (PRG pattern).
pub fn redirect_back(headers: &HeaderMap, fallback: &str, error: Option<&str>) -> SafeResponse {
    let base = headers.get("referer")
        .and_then(|v| v.to_str().ok())
        .map(|r| r.split('?').next().unwrap_or(r))
        .unwrap_or(fallback);
    let safe_base = OutputFactory::safe_redirect_url(&base, "/")
        .unwrap_or_else(|| fallback.to_string());
    match error {
        Some(msg) => SafeResponse::redirect(format!("{}?error={}", safe_base, crate::url_encode::url_encode(msg))),
        None => SafeResponse::redirect(safe_base),
    }
}

/// Extrage calea dintr-un URL complet: http://host/path?q → /path?q
pub fn extract_path_from_url(url: &str) -> String {
    if let Some(s) = url.find("://") {
        if let Some(p) = url[s + 3..].find('/') {
            return url[s + 3 + p..].to_string();
        }
    }
    if url.starts_with('/') {
        return url.to_string();
    }
    "/".to_string()
}

/// Extrage calea dintr-un Referer URL.
pub fn extract_path_from_referer(referer: &str) -> Option<String> {
    let path = referer.split("://").nth(1)?.split('/').skip(1).collect::<Vec<_>>().join("/");
    let path = format!("/{}", path);
    if path == "/" || path.starts_with("/?") { None } else { Some(path) }
}

/// Extrage user-ul curent din cookie (dacă e autentificat).
pub async fn current_user(
    headers: &HeaderMap,
    auth: &dyn rust_auth::AuthRepo,
) -> Option<rust_auth::UserPublic> {
    let token = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "token"))?;
    auth.verify_token(token).await.ok().map(|u| u.into())
}
