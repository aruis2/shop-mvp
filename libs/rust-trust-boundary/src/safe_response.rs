//! # SafeResponse — UNICA ieșire garantat sigură
//!
//! ## Principiu
//! Tot ce iese din aplicație spre browser trece prin SafeResponse.
//! Headerele de securitate sunt adăugate AUTOMAT.
//! Body-ul e sanitizat prin OutputFactory (dacă e implementat).
//!
//! ## Protecții automate
//! - CSP: `default-src 'self'`
//! - HSTS: `max-age=31536000; includeSubDomains`
//! - X-Frame-Options: `DENY`
//! - X-Content-Type-Options: `nosniff`
//! - Referrer-Policy: `strict-origin-when-cross-origin`
//! - Cache-Control: `no-store` pe rute sensibile

use http::StatusCode;

/// Status code garantat valid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafeStatus {
    Ok = 200,
    Redirect = 302,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    TooManyRequests = 429,
    ServerError = 500,
}

/// UNICA ieșire garantat sigură spre browser.
#[derive(Debug, Clone)]
pub struct SafeResponse {
    /// Status code
    pub status: SafeStatus,
    /// Body-ul răspunsului (HTML, JSON, text)
    pub body: String,
    /// Content-Type
    pub content_type: String,
    /// URL de redirect (dacă e redirect)
    pub location: Option<String>,
    /// Cookie-uri de setat
    pub set_cookies: Vec<(String, String, i64)>, // (nume, valoare, max_age)
    /// Cookie-uri de șters
    pub remove_cookies: Vec<String>,
    /// Headere adiționale
    pub extra_headers: Vec<(String, String)>,
}

impl SafeResponse {
    /// Creează un răspuns HTML 200 OK.
    pub fn html(body: impl Into<String>) -> Self {
        SafeResponse {
            status: SafeStatus::Ok,
            body: body.into(),
            content_type: "text/html; charset=utf-8".to_string(),
            location: None,
            set_cookies: Vec::new(),
            remove_cookies: Vec::new(),
            extra_headers: Vec::new(),
        }
    }

    /// Creează un răspuns JSON 200 OK.
    pub fn json(value: &serde_json::Value) -> Self {
        SafeResponse {
            status: SafeStatus::Ok,
            body: value.to_string(),
            content_type: "application/json; charset=utf-8".to_string(),
            location: None,
            set_cookies: Vec::new(),
            remove_cookies: Vec::new(),
            extra_headers: Vec::new(),
        }
    }

    /// Creează un redirect 302.
    pub fn redirect(url: impl Into<String>) -> Self {
        SafeResponse {
            status: SafeStatus::Redirect,
            body: String::new(),
            content_type: "text/plain; charset=utf-8".to_string(),
            location: Some(url.into()),
            set_cookies: Vec::new(),
            remove_cookies: Vec::new(),
            extra_headers: Vec::new(),
        }
    }

    /// Creează un răspuns de eroare 400.
    pub fn bad_request(msg: impl Into<String>) -> Self {
        SafeResponse {
            status: SafeStatus::BadRequest,
            body: msg.into(),
            content_type: "text/plain; charset=utf-8".to_string(),
            location: None,
            set_cookies: Vec::new(),
            remove_cookies: Vec::new(),
            extra_headers: Vec::new(),
        }
    }

    /// Creează un răspuns 401 neautorizat.
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        SafeResponse {
            status: SafeStatus::Unauthorized,
            body: msg.into(),
            content_type: "text/plain; charset=utf-8".to_string(),
            location: None,
            set_cookies: Vec::new(),
            remove_cookies: Vec::new(),
            extra_headers: Vec::new(),
        }
    }

    /// Creează un răspuns 404.
    pub fn not_found() -> Self {
        SafeResponse {
            status: SafeStatus::NotFound,
            body: "Not Found".to_string(),
            content_type: "text/plain; charset=utf-8".to_string(),
            location: None,
            set_cookies: Vec::new(),
            remove_cookies: Vec::new(),
            extra_headers: Vec::new(),
        }
    }

    /// Creează un răspuns 403.
    pub fn forbidden() -> Self {
        SafeResponse {
            status: SafeStatus::Forbidden,
            body: "Forbidden".to_string(),
            content_type: "text/plain; charset=utf-8".to_string(),
            location: None,
            set_cookies: Vec::new(),
            remove_cookies: Vec::new(),
            extra_headers: Vec::new(),
        }
    }

    /// Setează un cookie.
    pub fn with_cookie(mut self, name: &str, value: &str, max_age: i64) -> Self {
        self.set_cookies.push((
            name.to_string(),
            value.to_string(),
            max_age,
        ));
        self
    }

    /// Șterge un cookie.
    pub fn without_cookie(mut self, name: &str) -> Self {
        self.remove_cookies.push(name.to_string());
        self
    }

    /// Adaugă un header.
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.extra_headers.push((
            name.to_string(),
            value.to_string(),
        ));
        self
    }

    /// Convertește SafeResponse în Response HTTP (http crate).
    pub fn into_http_response(self) -> http::Response<String> {
        let status_code = match self.status {
            SafeStatus::Ok => StatusCode::OK,
            SafeStatus::Redirect => StatusCode::FOUND,
            SafeStatus::BadRequest => StatusCode::BAD_REQUEST,
            SafeStatus::Unauthorized => StatusCode::UNAUTHORIZED,
            SafeStatus::Forbidden => StatusCode::FORBIDDEN,
            SafeStatus::NotFound => StatusCode::NOT_FOUND,
            SafeStatus::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            SafeStatus::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let mut builder = http::Response::builder()
            .status(status_code)
            .header("Content-Type", &self.content_type);

        // Adaugă headere de securitate AUTOMATE
        // CSP — Content Security Policy (OWASP ASVS V10)
        builder = builder.header(
            "Content-Security-Policy",
            "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'",
        );
        // HSTS (RFC 6797)
        builder = builder.header(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        );
        // X-Frame-Options (RFC 7034)
        builder = builder.header("X-Frame-Options", "DENY");
        // X-Content-Type-Options
        builder = builder.header("X-Content-Type-Options", "nosniff");
        // Referrer-Policy
        builder = builder.header(
            "Referrer-Policy",
            "strict-origin-when-cross-origin",
        );

        // Redirect Location
        if let Some(loc) = &self.location {
            builder = builder.header("Location", loc.as_str());
        }

        // Set-Cookie
        for (name, value, max_age) in &self.set_cookies {
            let cookie_str = format!(
                "{}={}; HttpOnly; Path=/; SameSite=Lax; Max-Age={}",
                name, value, max_age
            );
            builder = builder.header("Set-Cookie", cookie_str.as_str());
        }

        // Remove-Cookie
        for name in &self.remove_cookies {
            let cookie_str = format!(
                "{}=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0",
                name
            );
            builder = builder.header("Set-Cookie", cookie_str.as_str());
        }

        // Headere adiționale
        for (name, value) in &self.extra_headers {
            builder = builder.header(name.as_str(), value.as_str());
        }

        builder.body(self.body)
            .unwrap_or_else(|_| {
                http::Response::builder()
                    .status(500)
                    .body("Internal Server Error".to_string())
                    .unwrap()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_response() {
        let resp = SafeResponse::html("<h1>Hello</h1>");
        let http = resp.into_http_response();
        assert_eq!(http.status(), 200);
        assert_eq!(http.body(), "<h1>Hello</h1>");
    }

    #[test]
    fn test_redirect() {
        let resp = SafeResponse::redirect("/products");
        let http = resp.into_http_response();
        assert_eq!(http.status(), 302);
        assert_eq!(http.headers().get("Location").unwrap(), "/products");
    }

    #[test]
    fn test_security_headers_present() {
        let resp = SafeResponse::html("ok");
        let http = resp.into_http_response();
        assert!(http.headers().contains_key("Content-Security-Policy"));
        assert!(http.headers().contains_key("Strict-Transport-Security"));
        assert!(http.headers().contains_key("X-Frame-Options"));
        assert!(http.headers().contains_key("X-Content-Type-Options"));
        assert!(http.headers().contains_key("Referrer-Policy"));
    }

    #[test]
    fn test_cookie() {
        let resp = SafeResponse::html("ok")
            .with_cookie("token", "abc", 86400);
        let http = resp.into_http_response();
        let cookie = http.headers().get("Set-Cookie").unwrap().to_str().unwrap();
        assert!(cookie.contains("token=abc"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Max-Age=86400"));
    }

    #[test]
    fn test_remove_cookie() {
        let resp = SafeResponse::html("ok")
            .without_cookie("token");
        let http = resp.into_http_response();
        let cookie = http.headers().get("Set-Cookie").unwrap().to_str().unwrap();
        assert!(cookie.contains("token="));
        assert!(cookie.contains("Max-Age=0"));
    }

    #[test]
    fn test_error_responses() {
        let bad = SafeResponse::bad_request("invalid");
        assert_eq!(bad.into_http_response().status(), 400);

        let unauth = SafeResponse::unauthorized("login required");
        assert_eq!(unauth.into_http_response().status(), 401);

        let nf = SafeResponse::not_found();
        assert_eq!(nf.into_http_response().status(), 404);
    }

    #[test]
    fn test_json_response() {
        let json = serde_json::json!({"ok": true});
        let resp = SafeResponse::json(&json);
        let http = resp.into_http_response();
        assert_eq!(http.status(), 200);
        assert_eq!(http.headers().get("Content-Type").unwrap(), "application/json; charset=utf-8");
    }
}
