// =============================================================================
// 🔑 Auth — capability: doar AuthRepo + RenderService
// =============================================================================

use axum::{
    extract::{Query, State},
};
use serde::Deserialize;

use std::sync::OnceLock;
use crate::state::AuthState;
use crate::render::DetectBasePath;
use crate::handlers::products::render_safe_json;
use crate::boundary::*;
use crate::types::parser::{parse_any_into, get_field};
use crate::url_encode::url_encode;
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
) -> SafeResponse {
    // Dacă e deja autentificat, redirect la home
    if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        if let Some(token) = crate::cookie::get_cookie(cookie, "token") {
            if s.auth.verify_token(token).await.is_ok() {
                let dest = q.redirect.clone().unwrap_or_else(|| format!("{}/", bp));
                return redirect_html(&dest);
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
    render_safe_json(&s.renderer, "auth/login.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn signup_page(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<AuthPageQuery>,
) -> SafeResponse {
    // Dacă e deja autentificat, redirect la home
    if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        if let Some(token) = crate::cookie::get_cookie(cookie, "token") {
            if s.auth.verify_token(token).await.is_ok() {
                let dest = q.redirect.clone().unwrap_or_else(|| format!("{}/", bp));
                return redirect_html(&dest);
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
    render_safe_json(&s.renderer, "auth/signup.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

/// Parsează body-ul ca JSON sau form-urlencoded
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
    // 🏭 InputFactory: validează email + password + name
    let (email_str, password, user_name) = parse_body_and_validate(body, |fields| {
        let email = InputFactory::parse_email(get_field(fields, "email")?)?;
        let password = get_field(fields, "password")?;
        // 🏭 InputFactory: numele e opțional, dacă e prezent trece prin InputFactory
        let name = match get_field(fields, "name") {
            Ok(s) if !s.trim().is_empty() => {
                Some(InputFactory::parse_name(s)?.as_str().to_string())
            }
            _ => None,
        };
        // 🔒 InputFactory: validare parolă (OWASP ASVS V2.1)
        if password.len() < 8 {
            return Err(InputError::PasswordTooShort);
        }
        if password.len() > 128 {
            return Err(InputError::PasswordTooLong);
        }
        if !password.chars().any(|c| c.is_uppercase()) {
            return Err(InputError::PasswordNoUppercase);
        }
        if !password.chars().any(|c| c.is_lowercase()) {
            return Err(InputError::PasswordNoLowercase);
        }
        if !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(InputError::PasswordNoDigit);
        }
        Ok((email.as_str().to_string(), password.to_string(), name))
    })?;

    let req = rust_auth::CreateUserRequest {
        email: email_str,
        password,
        name: user_name,
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

fn auth_response(resp: Result<(rust_auth::LoginResponse, String), String>, bp: &str) -> SafeResponse {
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
            SafeResponse::redirect(dest)
                .with_cookie("token", &r.token, 86400 * 7)
        }
        Err(e) => {
            tracing::error!(target: "auth::response", "auth eșuat: {}", e);
            SafeResponse::bad_request(e)
        }
    }
}

pub async fn signup_handler(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    body: String,
) -> SafeResponse {
    let ip = client_ip(&headers);
    if !rate_limiter().check(&ip) {
        debug_warn!(target: "auth::ratelimit", "Rate limit signup de la IP={}", ip);
        let err_enc = url_encode("Prea multe încercări. Încearcă din nou peste 1 minut.");
        let dest = format!("{}/signup?error={}", bp, err_enc);
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
            let err_enc = url_encode(&e);
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
            map.insert("is_authenticated".to_string(), serde_json::json!(true));
            map.insert("user_email".to_string(), serde_json::json!(u.email));
            map.insert("user_role".to_string(), serde_json::json!(u.role));
            if u.role == "admin" {
                map.insert("is_admin".to_string(), serde_json::json!(true));
            }
        }
    } else if let serde_json::Value::Object(map) = data {
        map.insert("is_authenticated".to_string(), serde_json::json!(false));
    }
}

/// 🔒 Redirect sigur — URL-ul trece prin OutputFactory::safe_redirect_url
fn safe_redirect(dest: &str, _bp: &str) -> SafeResponse {
    let safe = OutputFactory::safe_redirect_url(dest, "/")
        .unwrap_or_else(|| "/".to_string());
    SafeResponse::redirect(safe)
}

fn redirect_html(url: &str) -> SafeResponse {
    // 🔒 OutputFactory: validează URL-ul, previne XSS (javascript:) și open redirect
    // 🔒 Fără inline script (blocat de CSP script-src 'self'). Meta refresh e 100% HTML, nu JS.
    let safe_url = OutputFactory::safe_redirect_url(url, "/")
        .unwrap_or_else(|| "/".to_string());
    SafeResponse::html(format!(
        r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0;url={safe_url}"></head><body><p><a href="{safe_url}">Continuă</a></p></body></html>"#
    ))
}

/// GET /me — pagină HTML cu profilul utilizatorului
pub async fn me_handler(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
) -> SafeResponse {
    let token = match headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "token"))
    {
        Some(t) => t.to_string(),
        None => {
            let dest = format!("{}/login?error={}", bp, url_encode("Trebuie să fii autentificat"));
            return safe_redirect(&dest, &bp);
        }
    };

    let user = match s.auth.verify_token(&token).await {
        Ok(u) => u,
        Err(_) => {
            let dest = format!("{}/login?error={}", bp, url_encode("Token invalid"));
            return safe_redirect(&dest, &bp);
        }
    };

    let data = serde_json::json!({
        "title": "Profil — Shop MVP",
        "email": user.email,
        "name": user.name,
        "role": user.role,
    });

    render_safe_json(&s.renderer, "auth/me.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
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
) -> SafeResponse {
    // Determină URL-ul curent: ?redirect= → referer → /
    let redirect_val = q.redirect.clone();
    let current_url = redirect_val
        .or_else(|| {
            headers.get("referer")
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
    
    SafeResponse::redirect(safe_path).without_cookie("token")
}

pub async fn login_handler(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    body: String,
) -> SafeResponse {
    let ip = client_ip(&headers);
    if !rate_limiter().check(&ip) {
        debug_warn!(target: "auth::ratelimit", "Rate limit login de la IP={}", ip);
        let err_enc = url_encode("Prea multe încercări. Încearcă din nou peste 1 minut.");
        let dest = format!("{}/login?error={}", bp, err_enc);
        return safe_redirect(&dest, &bp);
    }
    
    // 🔒 ASVS L2: Account lockout — verifică dacă emailul de la acest IP e blocat
    let email = extract_email(&body);
    if let Some(email) = &email {
        if let Err(msg) = crate::check_lockout(&ip, email) {
            debug_warn!(target: "auth::lockout", "Cont blocat: {} de la IP={}", email, ip);
            let dest = format!("{}/login?error={}", bp, url_encode(msg));
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
            // Login reușit: resetează lockout per IP:email
            if let Some(email) = &email { crate::clear_lockout(&ip, email); }
            // NOTĂ: Coșul anonim NU se unește cu utilizatorul la login.
            // Itemele adăugate cât ești logat au deja user_id (vezi cart_add).
            auth_response(Ok((r, redirect)), &bp)
        }
        Err(e) => {
            // Login eșuat: înregistrează încercarea per IP:email
            if let Some(email) = &email { crate::record_failed_attempt(&ip, email); }
            debug_warn!(target: "auth::login", "login eșuat: {} redirect={}", e, redirect);
            let err_enc = url_encode(&e);
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
) -> SafeResponse {
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
                    safe_redirect(&dest, &bp).without_cookie("token")
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
) -> SafeResponse {
    let token = match headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "token"))
    {
        Some(t) => t.to_string(),
        None => return SafeResponse::unauthorized("Neautentificat"),
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
            SafeResponse::json(&data)
                .with_header("Content-Disposition", "attachment; filename=\"date-personale.json\"")
        }
        Err(_) => SafeResponse::unauthorized("Token invalid"),
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
) -> SafeResponse {
    let data = serde_json::json!({"title": "Politică de confidențialitate — Shop MVP"});
    render_safe_json(&s.renderer, "auth/privacy.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

/// GET /security — Politica de securitate (PCI DSS)
pub async fn security_policy_page(
    State(s): State<AuthState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
) -> SafeResponse {
    let data = serde_json::json!({"title": "Securitate — Shop MVP"});
    render_safe_json(&s.renderer, "auth/security.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}
