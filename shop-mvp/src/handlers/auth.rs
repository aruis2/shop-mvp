// =============================================================================
// 🔑 Auth — capability: doar AuthRepo + RenderService
// =============================================================================

use axum::{
    extract::{Query, State},
    http::HeaderMap,
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

#[derive(Deserialize)]
pub struct LogoutQuery {
    pub redirect: Option<String>,
}

// =============================================================================
// 📋 SignupForm — validat AUTOMAT de ValidatedForm (V8)
// =============================================================================

pub struct SignupForm {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub redirect: String,
}

impl ValidateForm for SignupForm {
    fn validate(fields: &[FormField], _headers: &HeaderMap) -> Result<Self, SafeResponse> {
        let email = InputFactory::parse_email(
            get_field(fields, "email").map_err(|_| SafeResponse::bad_request("Email lipsă"))?
        ).map_err(|e| SafeResponse::bad_request(e.to_string()))?;

        let password = get_field(fields, "password")
            .map_err(|_| SafeResponse::bad_request("Parola lipsește"))?
            .to_string();
        if password.len() < 8 { return Err(SafeResponse::bad_request("Parolă prea scurtă (min 8)")); }
        if password.len() > 128 { return Err(SafeResponse::bad_request("Parolă prea lungă (max 128)")); }
        if !password.chars().any(|c| c.is_uppercase()) { return Err(SafeResponse::bad_request("Parola trebuie să conțină o literă mare")); }
        if !password.chars().any(|c| c.is_lowercase()) { return Err(SafeResponse::bad_request("Parola trebuie să conțină o literă mică")); }
        if !password.chars().any(|c| c.is_ascii_digit()) { return Err(SafeResponse::bad_request("Parola trebuie să conțină o cifră")); }

        let name = match get_field(fields, "name") {
            Ok(s) if !s.trim().is_empty() => {
                Some(InputFactory::parse_name(s)
                    .map_err(|e| SafeResponse::bad_request(e.to_string()))?
                    .as_str().to_string())
            }
            _ => None,
        };
        let redirect = get_field(fields, "redirect").unwrap_or("").to_string();
        Ok(SignupForm { email: email.as_str().to_string(), password, name, redirect })
    }
}

async fn auth_signup(s: &AuthState, form: &SignupForm, referer: Option<&str>) -> Result<(rust_auth::LoginResponse, String), String> {
    let req = rust_auth::CreateUserRequest {
        email: form.email.clone(),
        password: form.password.clone(),
        name: form.name.clone(),
    };
    let redirect = if form.redirect.is_empty() {
        referer.and_then(|r| r.split('?').next()).unwrap_or("").to_string()
    } else {
        form.redirect.clone()
    };
    s.auth.signup(req).await.map(move |r| (r, redirect)).map_err(|e| e.to_string())
}

async fn auth_login(s: &AuthState, form: &LoginForm, referer: Option<&str>) -> Result<(rust_auth::LoginResponse, String), String> {
    let req = rust_auth::LoginRequest {
        email: form.email.clone(),
        password: form.password.clone(),
    };
    let redirect = if form.redirect.is_empty() {
        referer.and_then(|r| r.split('?').next()).unwrap_or("").to_string()
    } else {
        form.redirect.clone()
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
    ValidatedForm(form): ValidatedForm<SignupForm>,
) -> SafeResponse {
    let ip = client_ip(&headers);
    if !rate_limiter().check(&ip) {
        debug_warn!(target: "auth::ratelimit", "Rate limit signup de la IP={}", ip);
        let err_enc = url_encode("Prea multe încercări. Încearcă din nou peste 1 minut.");
        let dest = format!("{}/signup?error={}", bp, err_enc);
        return safe_redirect(&dest);
    }
    let referer = headers.get("referer").and_then(|v| v.to_str().ok());
    match auth_signup(&s, &form, referer).await {
        Ok((r, redirect)) => auth_response(Ok((r, redirect)), &bp),
        Err(e) => {
            debug_warn!(target: "auth::signup", "signup eșuat: {} redirect={}", e, form.redirect);
            let err_enc = url_encode(&e);
            let dest = if form.redirect.is_empty() {
                format!("{}/signup?error={}", bp, err_enc)
            } else {
                format!("{}/signup?error={}&redirect={}", bp, err_enc, form.redirect)
            };
            safe_redirect(&dest)
        }
    }
}



/// Extrage user-ul din cookie (fără a face redirect).
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
            return safe_redirect(&dest);
        }
    };

    let user = match s.auth.verify_token(&token).await {
        Ok(u) => u,
        Err(_) => {
            let dest = format!("{}/login?error={}", bp, url_encode("Token invalid"));
            return safe_redirect(&dest);
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
    ValidatedForm(form): ValidatedForm<LoginForm>,
) -> SafeResponse {
    let ip = client_ip(&headers);
    if !rate_limiter().check(&ip) {
        debug_warn!(target: "auth::ratelimit", "Rate limit login de la IP={}", ip);
        let err_enc = url_encode("Prea multe încercări. Încearcă din nou peste 1 minut.");
        let dest = format!("{}/login?error={}", bp, err_enc);
        return safe_redirect(&dest);
    }
    
    // 🔒 ASVS L2: Account lockout — verifică dacă emailul de la acest IP e blocat
    if let Err(msg) = crate::check_lockout(&ip, &form.email) {
        debug_warn!(target: "auth::lockout", "Cont blocat: {} de la IP={}", form.email, ip);
        let dest = format!("{}/login?error={}", bp, url_encode(msg));
        return safe_redirect(&dest);
    }
    
    let referer = headers.get("referer").and_then(|v| v.to_str().ok());
    match auth_login(&s, &form, referer).await {
        Ok((r, redirect)) => {
            // Login reușit: resetează lockout per IP:email
            crate::clear_lockout(&ip, &form.email);
            auth_response(Ok((r, redirect)), &bp)
        }
        Err(e) => {
            // Login eșuat: înregistrează încercarea per IP:email
            crate::record_failed_attempt(&ip, &form.email);
            debug_warn!(target: "auth::login", "login eșuat: {} redirect={}", e, form.redirect);
            let err_enc = url_encode(&e);
            let dest = if form.redirect.is_empty() {
                format!("{}/login?error={}", bp, err_enc)
            } else {
                format!("{}/login?error={}&redirect={}", bp, err_enc, form.redirect)
            };
            safe_redirect(&dest)
        }
    }
}

/// Extrage email-ul din body-ul formularului
// =============================================================================
// 📋 LoginForm — validat AUTOMAT de ValidatedForm (V8)
// =============================================================================

/// Formular login. Cîmpurile sunt validate prin InputFactory în ValidateForm.
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub redirect: String,
}

impl ValidateForm for LoginForm {
    fn validate(fields: &[FormField], _headers: &HeaderMap) -> Result<Self, SafeResponse> {
        let email = InputFactory::parse_email(
            get_field(fields, "email").map_err(|_| SafeResponse::bad_request("Email lipsă"))?
        ).map_err(|e| SafeResponse::bad_request(e.to_string()))?;
        
        let password = get_field(fields, "password")
            .map_err(|_| SafeResponse::bad_request("Parola lipsește"))?
            .to_string();
        if password.is_empty() {
            return Err(SafeResponse::bad_request("Parola lipsește"));
        }
        
        let redirect = get_field(fields, "redirect").unwrap_or("").to_string();
        Ok(LoginForm { email: email.as_str().to_string(), password, redirect })
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
            return safe_redirect(&dest);
        }
    };

    match s.auth.verify_token(&token).await {
        Ok(user) => {
            match s.auth.delete_user(user.id).await {
                Ok(_) => {
                    let dest = format!("{}/?success=Cont+șters", bp);
                    safe_redirect(&dest).without_cookie("token")
                }
                Err(e) => {
                    let err_msg = e.to_string().replace(' ', "+");
                    let dest = format!("{}/me?error=Eroare+la+ștergere:+{}", bp, err_msg);
                    safe_redirect(&dest)
                }
            }
        }
        Err(_) => {
            let dest = format!("{}/login?error=Token+invalid", bp);
            safe_redirect(&dest)
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
