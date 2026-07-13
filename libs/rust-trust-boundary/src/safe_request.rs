//! # SafeRequest — UNICUL tip de request garantat sigur
//!
//! ## Principiu
//! Tot ce vine de la browser trece prin TrustBoundary și iese ca SafeRequest.
//! Niciun handler nu mai primește `Request<Body>` sau extractoare Axum.
//!
//! ## Garanții
//! - `path` → SafePath (fără path traversal)
//! - `headers` → SafeHeaders (fără injection)
//! - `cookies` → SafeCookies (fără valori periculoase)
//! - `body` → SafeBody (parsat, validat, limitat)
//! - `client_ip` → din X-Forwarded-For sau "unknown"
//! - `method` → enum sigur (Get, Post, etc.)

use crate::safe_body::SafeBody;
use crate::safe_cookies::SafeCookies;
use crate::safe_headers::SafeHeaders;
use crate::safe_path::SafePath;

/// Metodă HTTP garantat sigură.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafeMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl SafeMethod {
    /// Returnează metoda ca string ("GET", "POST"...).
    pub fn as_str(&self) -> &'static str {
        match self {
            SafeMethod::Get => "GET",
            SafeMethod::Post => "POST",
            SafeMethod::Put => "PUT",
            SafeMethod::Delete => "DELETE",
            SafeMethod::Patch => "PATCH",
            SafeMethod::Head => "HEAD",
            SafeMethod::Options => "OPTIONS",
        }
    }

    /// Converteste din http::Method.
    pub fn from_method(m: &http::Method) -> Self {
        match m.as_str() {
            "GET" => SafeMethod::Get,
            "POST" => SafeMethod::Post,
            "PUT" => SafeMethod::Put,
            "DELETE" => SafeMethod::Delete,
            "PATCH" => SafeMethod::Patch,
            "HEAD" => SafeMethod::Head,
            "OPTIONS" => SafeMethod::Options,
            _ => SafeMethod::Get, // default safe
        }
    }

    /// Verifică dacă e GET.
    pub fn is_get(&self) -> bool {
        matches!(self, SafeMethod::Get)
    }

    /// Verifică dacă e POST.
    pub fn is_post(&self) -> bool {
        matches!(self, SafeMethod::Post)
    }
}

/// UNICUL tip de request pe care handlerele îl primesc.
///
/// Totul e garantat valid și sigur.
#[derive(Debug, Clone)]
pub struct SafeRequest {
    /// Metoda HTTP
    pub method: SafeMethod,
    /// Path-ul URL (garantat fără path traversal)
    pub path: SafePath,
    /// Query string-ul brut
    pub query_string: Option<String>,
    /// Headerele (garantat fără injection)
    pub headers: SafeHeaders,
    /// Cookie-urile (garantat valide)
    pub cookies: SafeCookies,
    /// Body-ul (parsat după Content-Type)
    pub body: SafeBody,
    /// IP-ul clientului (din X-Forwarded-For)
    pub client_ip: String,
    /// Request ID (trasabilitate)
    pub request_id: String,
    /// URL-ul site-ului (config)
    pub site_url: String,
}

impl SafeRequest {
    /// Creează un SafeRequest gol (pentru teste).
    pub fn empty() -> Self {
        let empty_headers = SafeHeaders::parse(&http::HeaderMap::new()).unwrap();
        let empty_cookies = SafeCookies::parse_from_header(None).unwrap();
        SafeRequest {
            method: SafeMethod::Get,
            path: SafePath::parse("/").unwrap(),
            query_string: None,
            headers: empty_headers,
            cookies: empty_cookies,
            body: SafeBody::Empty,
            client_ip: "unknown".to_string(),
            request_id: String::new(),
            site_url: String::new(),
        }
    }

    /// Verifică CSRF: potrivește Origin sau Referer cu site_url.
    pub fn verify_csrf(&self) -> bool {
        self.headers.verify_csrf(&self.site_url)
    }

    /// Generează un request ID pentru logare.
    pub fn generate_request_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("req_{:x}", now)
    }

    /// Returnează metoda ca string.
    pub fn method_str(&self) -> &'static str {
        self.method.as_str()
    }

    /// Returnează path-ul ca string.
    pub fn path_str(&self) -> &str {
        self.path.as_str()
    }

    /// Returnează token-ul JWT din cookie.
    pub fn auth_token(&self) -> Option<&str> {
        self.cookies.token()
    }

    /// Returnează session_id din cookie.
    pub fn session_id(&self) -> Option<&str> {
        self.cookies.session_id()
    }
}
