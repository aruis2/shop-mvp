// =============================================================================
// 🔑 Auth — capability: doar AuthRepo + RenderService
// =============================================================================

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use std::sync::OnceLock;
use crate::state::AuthState;
use crate::render::DetectBasePath;
use crate::handlers::products::render_or_err_json;
use crate::types::output::OutputFactory;
use crate::types::parser::{parse_any_into, get_field};
use crate::types::error::InputError;
use crate::types::InputFactory;
use crate::{debug_warn, debug_log};

/// Rate limiter pentru login/signup: 5 requesturi pe minut per IP
fn rate_limiter() -> &'static crate::ratelimit::RateLimiter {
    static RL: OnceLock<crate::ratelimit::RateLimiter> = OnceLock::new();
    RL.get_or_init(|| crate::ratelimit::RateLimiter::new(10, 60))
}

/// Extrage IP-ul clientului din headere (X-Forwarded-For) sau din conexiune
fn client_ip(headers: &axum::http::HeaderMap) -> String {
    headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[derive(Deserialize)]
pub struct AuthPageQuery {
    pub redirect: Option<String>,
    pub error: Option<String>,
}

pub async fn login_page(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<AuthPageQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    // Dacă e deja autentificat, redirect la home
    if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        if let Some(token) = crate::cookie::get_cookie(cookie, "token") {
            if s.auth.verify_token(token).await.is_ok() {
                let dest = q.redirect.clone().unwrap_or_else(|| format!("{}/", bp));
                return Ok(redirect_html(&dest));
            }
        }
    }
    let redirect = q.redirect.clone().or_else(|| {
        headers.get("referer")
            .and_then(|v| v.to_str().ok())
            .and_then(extract_path_from_referer)
    });
    tracing::warn!("login_page: q.redirect={:?} referer={:?} computed_redirect={:?}",
        q.redirect,
        headers.get("referer").and_then(|v| v.to_str().ok()),
        redirect);
    let mut data = serde_json::json!({
        "title": "Autentificare — Shop MVP",
    });
    if let Some(ref r) = redirect { data["redirect"] = serde_json::json!(r); }
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_or_err_json(&s.renderer, "auth/login.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn signup_page(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<AuthPageQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    // Dacă e deja autentificat, redirect la home
    if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        if let Some(token) = crate::cookie::get_cookie(cookie, "token") {
            if s.auth.verify_token(token).await.is_ok() {
                let dest = q.redirect.clone().unwrap_or_else(|| format!("{}/", bp));
                return Ok(redirect_html(&dest));
            }
        }
    }
    let redirect = q.redirect.or_else(|| {
        headers.get("referer")
            .and_then(|v| v.to_str().ok())
            .and_then(extract_path_from_referer)
    });
    let mut data = serde_json::json!({
        "title": "Înregistrare — Shop MVP",
    });
    if let Some(ref r) = redirect { data["redirect"] = serde_json::json!(r); }
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_or_err_json(&s.renderer, "auth/signup.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

/// Parsează body-ul ca JSON sau form-urlencoded (HTMX trimite form data)
/// și validează prin InputFactory
fn parse_body_and_validate<T>(
    body: &str,
    f: impl FnOnce(&[crate::types::parser::FormField]) -> Result<T, InputError>,
) -> Result<T, String> {
    parse_any_into(body, f).map_err(|e| e.to_string())
}

/// Extrage parametrul `redirect=` din raw body (form-urlencoded)
fn extract_redirect(body: &str) -> String {
    let fields = crate::types::parser::parse_form(body);
    crate::types::parser::get_field(&fields, "redirect")
        .map(|s| s.to_string())
        .unwrap_or_default()
}

#[derive(Deserialize)]
pub struct LogoutQuery {
    pub redirect: Option<String>,
}

async fn auth_signup(s: &AuthState, body: &str, referer: Option<&str>) -> Result<(rust_auth::LoginResponse, String), String> {
    // 🏭 InputFactory: validează email + password
    let (email_str, password) = parse_body_and_validate(body, |fields| {
        let email = InputFactory::parse_email(get_field(fields, "email")?)?;
        let password = get_field(fields, "password")?;
        // Password e string simplu, verificăm doar lungimea
        if password.len() < 8 {
            return Err(InputError::PasswordTooShort);
        }
        Ok((email.as_str().to_string(), password.to_string()))
    })?;

    let req = rust_auth::CreateUserRequest {
        email: email_str,
        password,
        name: None,
    };
    let redirect = extract_redirect(body);
    let redirect = if redirect.is_empty() {
        referer.and_then(|r| r.split('?').next()).unwrap_or("").to_string()
    } else {
        redirect
    };
    s.auth.signup(req).await.map(move |r| (r, redirect)).map_err(|e| e.to_string())
}

async fn auth_login(s: &AuthState, body: &str, referer: Option<&str>) -> Result<(rust_auth::LoginResponse, String), String> {
    // 🏭 InputFactory: validează email + password
    let (email_str, password) = parse_body_and_validate(body, |fields| {
        let email = InputFactory::parse_email(get_field(fields, "email")?)?;
        let password = get_field(fields, "password")?;
        if password.is_empty() {
            return Err(InputError::MissingField("password".to_string()));
        }
        Ok((email.as_str().to_string(), password.to_string()))
    })?;

    let req = rust_auth::LoginRequest {
        email: email_str,
        password,
    };
    let redirect = extract_redirect(body);
    let redirect = if redirect.is_empty() {
        referer.and_then(|r| r.split('?').next()).unwrap_or("").to_string()
    } else {
        redirect
    };
    s.auth.login(req).await.map(move |r| (r, redirect)).map_err(|e| e.to_string())
}

fn auth_response(resp: Result<(rust_auth::LoginResponse, String), String>, bp: &str) -> Response {
    match resp {
        Ok((r, redirect)) => {
            let raw_dest = if redirect.is_empty() || redirect == format!("{bp}/") {
                format!("{bp}/")
            } else {
                redirect
            };
            // 🔒 OutputFactory: validează URL-ul redirect
            let dest = OutputFactory::safe_redirect_url(&raw_dest, "/")
                .unwrap_or_else(|| format!("{bp}/"));
            debug_log!(target: "auth::response", "redirect: {} -> {}", raw_dest, dest);
            let cookie = crate::cookie::set_cookie("token", &r.token, 86400 * 7);
            let mut resp = (StatusCode::FOUND, [("Location", dest.as_str())]).into_response();
            resp.headers_mut().insert(
                axum::http::header::SET_COOKIE,
                axum::http::HeaderValue::from_str(&cookie).unwrap(),
            );
            resp
        }
        Err(e) => {
            tracing::error!(target: "auth::response", "auth eșuat: {}", e);
            (StatusCode::BAD_REQUEST, e).into_response()
        }
    }
}

pub async fn signup_handler(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let ip = client_ip(&headers);
    if !rate_limiter().check(&ip) {
        debug_warn!(target: "auth::ratelimit", "Rate limit signup de la IP={}", ip);
        let err_enc = "Prea multe încercări. Încearcă din nou peste 1 minut.";
        let dest = format!("{}/signup?error={}", bp, err_enc.replace(' ', "%20"));
        return safe_redirect(&dest, &bp);
    }
    let referer = headers.get("referer").and_then(|v| v.to_str().ok());
    let redirect = extract_redirect(&body);
    let redirect = if redirect.is_empty() {
        referer.and_then(|r| r.split('?').next()).unwrap_or("").to_string()
    } else {
        redirect
    };
    match auth_signup(&s, &body, referer).await {
        Ok((r, _)) => auth_response(Ok((r, redirect)), &bp),
        Err(e) => {
            debug_warn!(target: "auth::signup", "signup eșuat: {} redirect={}", e, redirect);
            let err_enc = e.replace(' ', "%20");
            let dest = if redirect.is_empty() {
                format!("{}/signup?error={}", bp, err_enc)
            } else {
                format!("{}/signup?error={}&redirect={}", bp, err_enc, redirect)
            };
            safe_redirect(&dest, &bp)
        }
    }
}

fn extract_path_from_url(url: &str) -> String {
    // Dacă e URL complet (http://...), extrage doar calea
    if let Some(s) = url.find("://") {
        if let Some(p) = url[s + 3..].find('/') {
            let path = &url[s + 3 + p..];
            return path.to_string(); // păstrăm query params
        }
    }
    // Dacă e deja cale (/cart, /search?q=s22), returneaz-o direct
    if url.starts_with('/') {
        return url.to_string();
    }
    "/".to_string()
}

/// Pagină HTML simplă cu redirect via meta refresh + JS + ștergere localStorage
/// Extrage user-ul din cookie (fără a face redirect). Returnează None dacă nu e autentificat.
pub async fn current_user(
    headers: &axum::http::HeaderMap,
    auth: &dyn rust_auth::AuthRepo,
) -> Option<rust_auth::UserPublic> {
    let token = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "token"))?;
    auth.verify_token(token).await.ok().map(|u| u.into())
}

/// Adaugă user-ul în contextul Tera (dacă e autentificat), sub cheia "user"
pub async fn inject_user_ctx(
    ctx: &mut tera::Context,
    headers: &axum::http::HeaderMap,
    auth: &dyn rust_auth::AuthRepo,
) {
    if let Some(u) = current_user(headers, auth).await {
        ctx.insert("user_email", &u.email);
        ctx.insert("user_role", &u.role);
        if u.role == "admin" {
            ctx.insert("is_admin", &true);
        }
    }
}

/// 🔒 Versiunea JSON a inject_user_ctx — pentru render_json().
/// Injectează user info într-un serde_json::Value în loc de Tera Context.
pub async fn inject_user_ctx_json(
    data: &mut serde_json::Value,
    headers: &axum::http::HeaderMap,
    auth: &dyn rust_auth::AuthRepo,
) {
    if let Some(u) = current_user(headers, auth).await {
        if let serde_json::Value::Object(map) = data {
            map.insert("user_email".to_string(), serde_json::json!(u.email));
            map.insert("user_role".to_string(), serde_json::json!(u.role));
            if u.role == "admin" {
                map.insert("is_admin".to_string(), serde_json::json!(true));
            }
        }
    }
}

/// 🔒 Redirect sigur — URL-ul trece prin OutputFactory::safe_redirect_url
fn safe_redirect(dest: &str, _bp: &str) -> Response {
    let safe = OutputFactory::safe_redirect_url(dest, "/")
        .unwrap_or_else(|| "/".to_string());
    (StatusCode::FOUND, [("Location", safe)]).into_response()
}

fn redirect_html(url: &str) -> Html<String> {
    // 🔒 OutputFactory: validează URL-ul, previne XSS (javascript:) și open redirect
    let safe_url = OutputFactory::safe_redirect_url(url, "/")
        .unwrap_or_else(|| "/".to_string());
    Html(format!(
        r#"<!DOCTYPE html><html><head><meta http-equiv=\"refresh\" content=\"0;url={safe_url}\"></head><body><script>localStorage.clear();window.location.href='{safe_url}';</script></body></html>"#
    ))
}

/// GET /me — returnează user-ul din token cookie (pentru restaurare localStorage)
pub async fn me_handler(
    State(s): State<AuthState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let token = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "token"))
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Neautentificat".to_string()))?;
    
    let user = s.auth.verify_token(token).await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "name": user.name,
        "role": user.role
    })))
}

/// Extrage calea dintr-un Referer URL: http://host/path → Some("/path")
fn extract_path_from_referer(referer: &str) -> Option<String> {
    let path = referer.split("://").nth(1)?.split('/').skip(1).collect::<Vec<_>>().join("/");
    let path = format!("/{}", path);
    // Păstrăm query params (ex: /search?q=s22 → /search?q=s22)
    if path == "/" || path.starts_with("/?") { None } else { Some(path) }
}

pub async fn logout_handler(
    headers: axum::http::HeaderMap,
    Query(q): Query<LogoutQuery>,
) -> Response {
    let is_htmx = headers.get("hx-request").is_some();
    let cookie = crate::cookie::remove_cookie("token");
    
    // Determină URL-ul curent: ?redirect= → hx-current-url → referer → /
    let redirect_val = q.redirect.clone();
    let current_url = redirect_val
        .or_else(|| {
            headers.get("hx-current-url")
                .or_else(|| headers.get("referer"))
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        });
    
    let current_path = current_url.as_deref()
        .map(extract_path_from_url)
        .unwrap_or_else(|| "/".to_string());
    
    // 🔒 OutputFactory: validează URL-ul (previne open redirect)
    let safe_path = OutputFactory::safe_redirect_url(&current_path, "/")
        .unwrap_or_else(|| "/".to_string());
    
    debug_log!(target: "auth::logout", "logout: redirect={:?} -> path={} safe={}", q.redirect, current_path, safe_path);
    
    let mut resp = if is_htmx {
        let mut r = (StatusCode::OK, Html(String::new())).into_response();
        let header_val = OutputFactory::safe_header_value(&safe_path);
        r.headers_mut().insert(
            axum::http::HeaderName::from_static("hx-redirect"),
            axum::http::HeaderValue::from_str(&header_val).unwrap(),
        );
        r
    } else {
        let r = (StatusCode::FOUND, [("Location", safe_path.as_str())]).into_response();
        r
    };
    resp.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&cookie).unwrap(),
    );
    resp
}

pub async fn login_handler(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let ip = client_ip(&headers);
    if !rate_limiter().check(&ip) {
        debug_warn!(target: "auth::ratelimit", "Rate limit login de la IP={}", ip);
        let err_enc = "Prea multe încercări. Încearcă din nou peste 1 minut.";
        let dest = format!("{}/login?error={}", bp, err_enc.replace(' ', "%20"));
        return safe_redirect(&dest, &bp);
    }
    
    // 🔒 ASVS L2: Account lockout - verifică dacă emailul e blocat
    let email = extract_email(&body);
    if let Some(email) = &email {
        if let Err(msg) = crate::check_lockout(email) {
            debug_warn!(target: "auth::lockout", "Cont blocat: {}", email);
            let dest = format!("{}/login?error={}", bp, msg.replace(' ', "%20"));
            return safe_redirect(&dest, &bp);
        }
    }
    
    let referer = headers.get("referer").and_then(|v| v.to_str().ok());
    let redirect = extract_redirect(&body);
    let redirect = if redirect.is_empty() {
        referer.and_then(|r| r.split('?').next()).unwrap_or("").to_string()
    } else {
        redirect
    };
    match auth_login(&s, &body, referer).await {
        Ok((r, _)) => {
            // Login reușit: resetează lockout
            if let Some(email) = &email { crate::clear_lockout(email); }
            auth_response(Ok((r, redirect)), &bp)
        }
        Err(e) => {
            // Login eșuat: înregistrează încercarea
            if let Some(email) = &email { crate::record_failed_attempt(email); }
            debug_warn!(target: "auth::login", "login eșuat: {} redirect={}", e, redirect);
            let err_enc = e.replace(' ', "%20");
            let dest = if redirect.is_empty() {
                format!("{}/login?error={}", bp, err_enc)
            } else {
                format!("{}/login?error={}&redirect={}", bp, err_enc, redirect)
            };
            safe_redirect(&dest, &bp)
        }
    }
}

/// Extrage email-ul din body-ul formularului
fn extract_email(body: &str) -> Option<String> {
    if let Ok(form) = serde_urlencoded::from_str::<std::collections::HashMap<String, String>>(body) {
        form.get("email").cloned()
    } else {
        None
    }
}

// =============================================================================
// 🔒 GDPR: Ștergere cont (Dreptul la ștergere - Art. 17)
// =============================================================================

/// POST /account/delete — Șterge contul utilizatorului (anonimizează datele)
pub async fn delete_account_handler(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
) -> Response {
    // Verifică autentificarea
    let token = match headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "token"))
    {
        Some(t) => t.to_string(),
        None => {
            let dest = format!("{}/login?error=Autentifică-te+întâi", bp);
            return safe_redirect(&dest, &bp);
        }
    };

    match s.auth.verify_token(&token).await {
        Ok(user) => {
            match s.auth.delete_user(user.id).await {
                Ok(_) => {
                    let dest = format!("{}/?success=Cont+șters", bp);
                    let mut resp = safe_redirect(&dest, &bp);
                    resp.headers_mut().insert(
                        axum::http::header::HeaderName::from_static("set-cookie"),
                        axum::http::HeaderValue::from_static("token=; Max-Age=0; Path=/"),
                    );
                    resp
                }
                Err(e) => {
                    let err_msg = e.to_string().replace(' ', "+");
                    let dest = format!("{}/me?error=Eroare+la+ștergere:+{}", bp, err_msg);
                    safe_redirect(&dest, &bp)
                }
            }
        }
        Err(_) => {
            let dest = format!("{}/login?error=Token+invalid", bp);
            safe_redirect(&dest, &bp)
        },
    }
}

/// GET /account/export — Exportă toate datele utilizatorului în format JSON
pub async fn export_data_handler(
    State(s): State<AuthState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let token = match headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "token"))
    {
        Some(t) => t.to_string(),
        None => return (StatusCode::UNAUTHORIZED, "Neautentificat").into_response(),
    };

    match s.auth.verify_token(&token).await {
        Ok(user) => {
            use serde_json::json;
            let data = json!({
                "user": {
                    "id": user.id,
                    "email": user.email,
                    "name": user.name,
                    "role": user.role,
                },
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "note": "Aceste date au fost exportate conform GDPR Art. 20."
            });
            (StatusCode::OK, [
                ("Content-Type", "application/json; charset=utf-8"),
                ("Content-Disposition", "attachment; filename=\"date-personale.json\""),
            ], serde_json::to_string_pretty(&data).unwrap_or_default()).into_response()
        }
        Err(_) => (StatusCode::UNAUTHORIZED, "Token invalid").into_response(),
    }
}

// =============================================================================
// 🔒 GDPR: Politică de confidențialitate
// =============================================================================

/// GET /privacy — Pagină simplă cu politica de confidențialitate
pub async fn privacy_policy_page(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let data = serde_json::json!({"title": "Politică de confidențialitate — Shop MVP"});
    render_or_err_json(&s.renderer, "auth/privacy.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

/// GET /security — Politica de securitate (PCI DSS)
pub async fn security_policy_page(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let data = serde_json::json!({"title": "Securitate — Shop MVP"});
    render_or_err_json(&s.renderer, "auth/security.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}
