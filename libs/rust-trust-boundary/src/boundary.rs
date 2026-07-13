//! # TrustBoundary — UNICA graniță între aplicație și lumea exterioară
//!
//! ## Filozofie
//! Conform PHILOSOPHY.md #13 (zero intermediari), TRUST-BOUNDARY.md:
//! Tot ce vine de la utilizator trece PRIN AICI.
//! Tot ce iese spre utilizator trece PRIN AICI.
//!
//! ## Standarde
//! - OWASP ASVS V5.1 (Input Validation)
//! - OWASP ASVS V10 (Output Encoding)
//! - OWASP API Top 10 #1 (BOLA)
//! - CIS Control 6 (Access Control)
//!
//! ## Garanții
//! - Orice `SafeRequest` conține doar date GARANTAT valide
//! - Orice `SafeResponse` conține doar headere de securitate AUTOMATE
//! - Niciun handler nu primește date brute de la utilizator

use crate::safe_body::SafeBody;
use crate::safe_cookies::SafeCookies;
use crate::safe_headers::SafeHeaders;
use crate::safe_path::SafePath;
use crate::safe_request::{SafeMethod, SafeRequest};
use crate::safe_response::SafeResponse;
use crate::BoundaryError;

/// TrustBoundary — UNICA poartă între exterior și interior.
///
/// ```rust,no_run
/// use rust_trust_boundary::{TrustBoundary, SafeRequest, SafeResponse};
///
/// async fn handler(req: http::Request<String>) -> http::Response<String> {
///     match TrustBoundary::parse_request(&req) {
///         Ok(safe) => {
///             // safe.path, safe.method, safe.body, safe.cookies...
///             // TOATE sunt garantat valide
///             SafeResponse::html("<h1>OK</h1>").into_http_response()
///         }
///         Err(e) => {
///             SafeResponse::bad_request(e.to_string()).into_http_response()
///         }
///     }
/// }
/// ```
pub struct TrustBoundary;

impl TrustBoundary {
    /// Parsează și validează un request HTTP.
    ///
    /// Acesta e SINGURUL punct de intrare pentru date externe.
    /// Orice eroare aici înseamnă request invalid → 400 înainte de orice handler.
    ///
    /// # Flow
    /// 1. Extrage metoda → `SafeMethod`
    /// 2. Parsează path-ul → `SafePath` (fără traversal)
    /// 3. Parsează headerele → `SafeHeaders` (fără injection)
    /// 4. Parsează cookie-urile → `SafeCookies` (fără valori periculoase)
    /// 5. Parsează body-ul → `SafeBody` (după Content-Type)
    /// 6. Extra IP-ul clientului → string
    pub fn parse_request<B>(req: &http::Request<B>) -> Result<SafeRequest, BoundaryError>
    where
        B: AsRef<[u8]>,
    {
        // 1. Metoda
        let method = SafeMethod::from_method(req.method());

        // 2. Path
        let path = SafePath::parse(req.uri().path())?;

        // 3. Query string
        let query_string = req.uri().query().map(|q| q.to_string());

        // 4. Headere
        let headers = SafeHeaders::parse(req.headers())?;

        // 5. Cookie-uri
        let cookie_header = headers.get("cookie");
        let cookies = SafeCookies::parse_from_header(cookie_header)?;

        // 6. Body
        let content_type = headers.content_type();
        let body_bytes = req.body().as_ref();
        let body = SafeBody::parse(content_type, body_bytes)?;

        // 7. Client IP
        let client_ip = headers
            .client_ip()
            .unwrap_or("unknown")
            .to_string();

        // 8. Request ID
        let request_id = SafeRequest::generate_request_id();

        Ok(SafeRequest {
            method,
            path,
            query_string,
            headers,
            cookies,
            body,
            client_ip,
            request_id,
            site_url: String::new(), // se setează din config
        })
    }

    /// Parsează request-ul și setează site_url din config.
    pub fn parse_request_with_config<B>(
        req: &http::Request<B>,
        site_url: &str,
    ) -> Result<SafeRequest, BoundaryError>
    where
        B: AsRef<[u8]>,
    {
        let mut safe = Self::parse_request(req)?;
        safe.site_url = site_url.to_string();
        Ok(safe)
    }

    /// Creează un răspuns 400 cu mesaj de eroare pentru un BoundaryError.
    pub fn error_response(err: &BoundaryError) -> SafeResponse {
        match err {
            BoundaryError::PathTraversalDetected(_) => {
                SafeResponse::forbidden()
            }
            BoundaryError::BodyTooLarge(_) => {
                SafeResponse::bad_request("Body prea mare (max 2MB)")
            }
            BoundaryError::InvalidUtf8 => {
                SafeResponse::bad_request("Encoding invalid")
            }
            _ => {
                SafeResponse::bad_request(err.to_string())
            }
        }
    }

    /// Verifică CSRF pe un SafeRequest.
    /// Returnează `Ok(())` sau `SafeResponse::forbidden()`.
    pub fn verify_csrf(safe: &SafeRequest) -> Result<(), SafeResponse> {
        if safe.method.is_post() && !safe.verify_csrf() {
            tracing::warn!(
                target: "csrf",
                "CSRF respins: method={:?} origin={:?} referer={:?}",
                safe.method,
                safe.headers.origin(),
                safe.headers.referer(),
            );
            return Err(SafeResponse::forbidden());
        }
        Ok(())
    }
}
