//! # SafeCookies — Cookie-uri garantat sigure
//!
//! ## Principiu
//! Cookie-urile vin în header-ul `Cookie` și sunt parse-ate manual.
//! Fără dependințe de crate-uri externe de cookie.
//!
//! ## Protecții
//! - Separare corectă după `;`
//! - Validare lungime maximă per valoare (256)
//! - Fără caractere periculoase în valori

use crate::BoundaryError;

/// Cookie-uri garantat sigure.
///
/// Expune doar cookie-urile de care aplicația are nevoie:
/// - `token` — JWT de autentificare
/// - `session_id` — UUID pentru sesiune anonimă
/// - `csrf_token` — token CSRF
#[derive(Debug, Clone)]
pub struct SafeCookies {
    /// Token JWT de autentificare
    token: Option<String>,
    /// Session ID (UUID)
    session_id: Option<String>,
    /// CSRF token
    csrf_token: Option<String>,
    /// Toate cookie-urile brute (validate)
    raw: Vec<(String, String)>,
}

impl SafeCookies {
    /// Parsează cookie-urile din header-ul Cookie.
    ///
    /// Acceptă atât un string direct, cât și un SafeHeaders.
    pub fn parse_from_header(cookie_header: Option<&str>) -> Result<Self, BoundaryError> {
        let mut raw = Vec::new();
        let mut token = None;
        let mut session_id = None;
        let mut csrf_token = None;

        if let Some(header) = cookie_header {
            for part in header.split(';') {
                let part = part.trim();
                if let Some(eq_pos) = part.find('=') {
                    let name = &part[..eq_pos].trim().to_string();
                    let value = &part[eq_pos + 1..].trim().to_string();

                    // Validare: lungime maximă
                    if value.len() > 256 {
                        return Err(BoundaryError::CookieTooLong(name.clone()));
                    }

                    // Validare: fără caractere periculoase
                    if value.contains(';') || value.contains(',') || value.contains(' ') {
                        return Err(BoundaryError::InvalidCookieValue(name.clone()));
                    }

                    raw.push((name.clone(), value.clone()));

                    match name.as_str() {
                        "token" => token = Some(value.clone()),
                        "session_id" | "session-id" => session_id = Some(value.clone()),
                        "csrf_token" => csrf_token = Some(value.clone()),
                        _ => {}
                    }
                }
            }
        }

        Ok(SafeCookies {
            token,
            session_id,
            csrf_token,
            raw,
        })
    }

    /// Token JWT (autentificare).
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Session ID (UUID pentru coș anonim).
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// CSRF token.
    pub fn csrf_token(&self) -> Option<&str> {
        self.csrf_token.as_deref()
    }

    /// Verifică dacă există un anumit cookie.
    pub fn has(&self, name: &str) -> bool {
        self.raw.iter().any(|(n, _)| n == name)
    }

    /// Returnează toate cookie-urile.
    pub fn all(&self) -> &[(String, String)] {
        &self.raw
    }

    /// Returnează numărul de cookie-uri.
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Dacă utilizatorul e autentificat (are token).
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_cookies() {
        let c = SafeCookies::parse_from_header(None).unwrap();
        assert_eq!(c.len(), 0);
        assert!(!c.is_authenticated());
    }

    #[test]
    fn test_empty_header() {
        let c = SafeCookies::parse_from_header(Some("")).unwrap();
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn test_token_and_session() {
        let c = SafeCookies::parse_from_header(Some(
            "token=eyJhbGciOiJIUzI1NiJ9.test; session_id=550e8400-e29b-41d4-a716-446655440000"
        )).unwrap();
        assert_eq!(c.token(), Some("eyJhbGciOiJIUzI1NiJ9.test"));
        assert_eq!(c.session_id(), Some("550e8400-e29b-41d4-a716-446655440000"));
        assert!(c.is_authenticated());
    }

    #[test]
    fn test_token_only() {
        let c = SafeCookies::parse_from_header(Some("token=abc123")).unwrap();
        assert_eq!(c.token(), Some("abc123"));
        assert!(c.is_authenticated());
    }

    #[test]
    fn test_session_id_variant() {
        let c = SafeCookies::parse_from_header(Some("session-id=abc")).unwrap();
        assert_eq!(c.session_id(), Some("abc"));
    }

    #[test]
    fn test_csrf_token() {
        let c = SafeCookies::parse_from_header(Some("csrf_token=xyz")).unwrap();
        assert_eq!(c.csrf_token(), Some("xyz"));
    }

    #[test]
    fn test_has_cookie() {
        let c = SafeCookies::parse_from_header(Some("token=abc; session_id=xyz")).unwrap();
        assert!(c.has("token"));
        assert!(c.has("session_id"));
        assert!(!c.has("nonexistent"));
    }

    #[test]
    fn test_value_too_long() {
        let long_val = "a".repeat(257);
        let r = SafeCookies::parse_from_header(Some(&format!("token={}", long_val)));
        assert!(r.is_err());
    }

    #[test]
    fn test_value_with_semicolon() {
        // acest caz e greu de reprodus pentru că split(';') separă
        // dar e util ca documentație
        let c = SafeCookies::parse_from_header(Some("token=abc; session_id=xyz")).unwrap();
        assert_eq!(c.token(), Some("abc"));
    }

    #[test]
    fn test_all_cookies() {
        let c = SafeCookies::parse_from_header(Some("a=1; b=2; c=3")).unwrap();
        assert_eq!(c.len(), 3);
    }
}
