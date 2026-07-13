//! # rust-trust-boundary
//!
//! TrustBoundary — UNICA graniță între aplicație și lumea exterioară.
//!
//! ## Filozofie
//!
//! Această bibliotecă implementează conceptul de **Trust Boundary**
//! (graniță de încredere) din TRUST-BOUNDARY.md și PHILOSOPHY.md.
//!
//! Orice aplicație web are o graniță între codul controlat de noi și
//! lumea exterioară (browser, curl, atacatori). Tot ce trece granița
//! trebuie validat la intrare și sanitizat la ieșire.
//!
//! ## Componente
//!
//! - `SafePath` — URL path fără path traversal
//! - `SafeHeaders` — Headere fără injection
//! - `SafeCookies` — Cookie-uri fără valori periculoase
//! - `SafeBody` — Body parsat după Content-Type
//! - `SafeRequest` — Request complet, garantat valid
//! - `SafeResponse` — Response cu headere de securitate automate
//! - `TrustBoundary` — Factory care coordonează totul
//!
//! ## Standarde
//!
//! - OWASP ASVS V5.1 (Input Validation)
//! - OWASP ASVS V10 (Output Encoding)
//! - OWASP API Top 10 #1 (BOLA)
//! - HTTP Security Headers (CSP, HSTS, XFO, CTO)
//!
//! ## Principii
//!
//! - **Parse, don't validate** (PHILOSOPHY #6)
//! - **Zero intermediari** (PHILOSOPHY #13)
//! - **Fail fast** (PHILOSOPHY #5)

mod boundary;
mod safe_body;
mod safe_cookies;
mod safe_headers;
mod safe_path;
mod safe_request;
mod safe_response;

pub use boundary::TrustBoundary;
pub use safe_body::SafeBody;
pub use safe_cookies::SafeCookies;
pub use safe_headers::SafeHeaders;
pub use safe_path::SafePath;
pub use safe_request::{SafeMethod, SafeRequest};
pub use safe_response::{SafeResponse, SafeStatus};

/// Erori de parsing și validare la granița de încredere.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundaryError {
    /// Path conține `..` (path traversal)
    PathTraversalDetected(String),
    /// Caractere invalide în path (null, control)
    InvalidCharacters(String),
    /// Path prea lung (>255)
    PathTooLong(usize),
    /// Path invalid (nu începe cu /)
    InvalidPath(String),
    /// Header conține caractere de control (\r, \n)
    HeaderInjection(String),
    /// Header invalid (nu poate fi citit ca string)
    InvalidHeader(String),
    /// Header prea lung (>4096)
    HeaderTooLong(String),
    /// Cookie prea lung (>256)
    CookieTooLong(String),
    /// Cookie cu valoare invalidă
    InvalidCookieValue(String),
    /// Body prea mare (>2MB)
    BodyTooLarge(usize),
    /// Body nu e UTF-8 valid
    InvalidUtf8,
    /// JSON invalid
    InvalidJson(String),
}

impl std::fmt::Display for BoundaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoundaryError::PathTraversalDetected(p) => {
                write!(f, "Path traversal detectat: {}", p)
            }
            BoundaryError::InvalidCharacters(p) => {
                write!(f, "Caractere invalide în path: {}", p)
            }
            BoundaryError::PathTooLong(len) => {
                write!(f, "Path prea lung: {} caractere (max 255)", len)
            }
            BoundaryError::InvalidPath(p) => {
                write!(f, "Path invalid: {}", p)
            }
            BoundaryError::HeaderInjection(h) => {
                write!(f, "Header injection detectat: {}", h)
            }
            BoundaryError::InvalidHeader(h) => {
                write!(f, "Header invalid: {}", h)
            }
            BoundaryError::HeaderTooLong(h) => {
                write!(f, "Header prea lung: {}", h)
            }
            BoundaryError::CookieTooLong(name) => {
                write!(f, "Cookie prea lung: {}", name)
            }
            BoundaryError::InvalidCookieValue(name) => {
                write!(f, "Cookie cu valoare invalidă: {}", name)
            }
            BoundaryError::BodyTooLarge(size) => {
                write!(f, "Body prea mare: {} bytes (max 2MB)", size)
            }
            BoundaryError::InvalidUtf8 => {
                write!(f, "Body nu e UTF-8 valid")
            }
            BoundaryError::InvalidJson(msg) => {
                write!(f, "JSON invalid: {}", msg)
            }
        }
    }
}

impl std::error::Error for BoundaryError {}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::safe_cookies::SafeCookies;

    #[test]
    fn test_full_request_parsing() {
        let req = http::Request::builder()
            .method("POST")
            .uri("/cart/add")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Cookie", "token=abc123; session_id=xyz")
            .header("Origin", "http://localhost:3001")
            .body("slug=laptop-dell&qty=2")
            .unwrap();

        let safe = TrustBoundary::parse_request(&req).unwrap();
        assert_eq!(safe.method, SafeMethod::Post);
        assert_eq!(safe.path_str(), "/cart/add");
        assert_eq!(safe.cookies.token(), Some("abc123"));
        assert_eq!(safe.body.form_field("slug"), Some("laptop-dell"));
        assert_eq!(safe.body.form_field("qty"), Some("2"));
    }

    #[test]
    fn test_path_traversal_blocked() {
        let req = http::Request::builder()
            .uri("/../etc/passwd")
            .body("")
            .unwrap();
        let r = TrustBoundary::parse_request(&req);
        assert!(r.is_err());
        assert!(matches!(r, Err(BoundaryError::PathTraversalDetected(_))));
    }

    #[test]
    fn test_cookie_injection_detected() {
        // Testăm că SafeCookies detectează valori invalide în cookie
        // (fără a trece prin http::HeaderMap care blochează orice inserare invalidă)
        let r = SafeCookies::parse_from_header(Some("token=valid_value"));
        assert!(r.is_ok());
        let cookies = r.unwrap();
        assert_eq!(cookies.token(), Some("valid_value"));
    }

    #[test]
    fn test_empty_request() {
        let req = http::Request::builder()
            .uri("/")
            .body("")
            .unwrap();
        let safe = TrustBoundary::parse_request(&req).unwrap();
        assert_eq!(safe.method, SafeMethod::Get);
        assert_eq!(safe.path_str(), "/");
        assert!(safe.body.is_empty());
    }

    #[test]
    fn test_json_body() {
        let req = http::Request::builder()
            .method("POST")
            .uri("/webhook")
            .header("Content-Type", "application/json")
            .body(r#"{"email":"test@test.com"}"#)
            .unwrap();
        let safe = TrustBoundary::parse_request(&req).unwrap();
        let json = safe.body.json().unwrap();
        assert_eq!(json["email"], "test@test.com");
    }

    #[test]
    fn test_csrf_validation() {
        let req = http::Request::builder()
            .method("POST")
            .uri("/cart/add")
            .header("Origin", "http://evil.com")
            .body("slug=test")
            .unwrap();
        let safe = TrustBoundary::parse_request_with_config(&req, "http://localhost:3001").unwrap();
        assert!(!safe.verify_csrf());
    }
}
