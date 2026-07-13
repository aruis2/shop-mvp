//! # SafeBody — Body HTTP garantat sigur
//!
//! ## Principiu
//! Body-ul e parsat în funcție de Content-Type.
//! Suportă form URL-encoded (implicit) și JSON.
//!
//! ## Protecții
//! - Limită de dimensiune (2MB)
//! - Parsare manuală URL-encoded (zero dependințe)
//! - Validare UTF-8

use crate::BoundaryError;
use std::collections::HashMap;

/// Body HTTP garantat sigur.
///
/// Conține câmpurile parse-ate după Content-Type.
/// Pentru `application/x-www-form-urlencoded` — campuri.
/// Pentru `application/json` — JSON value.
/// Pentru `text/plain` — text brut.
#[derive(Debug, Clone)]
pub enum SafeBody {
    /// Form URL-encoded — câmpuri cheie=valoare
    Form(HashMap<String, String>),
    /// JSON — valoare JSON parse-ata
    Json(serde_json::Value),
    /// Text simplu
    Text(String),
    /// Body gol
    Empty,
}

impl SafeBody {
    /// Parsează body-ul în funcție de Content-Type.
    ///
    /// # Argumente
    /// - `content_type` — header-ul Content-Type (lowercase)
    /// - `body` — bytes-urile body-ului
    pub fn parse(content_type: Option<&str>, body: &[u8]) -> Result<Self, BoundaryError> {
        // Limită de dimensiune: 2MB
        if body.len() > 2 * 1024 * 1024 {
            return Err(BoundaryError::BodyTooLarge(body.len()));
        }

        let body_str = std::str::from_utf8(body)
            .map_err(|_| BoundaryError::InvalidUtf8)?;

        if body_str.is_empty() {
            return Ok(SafeBody::Empty);
        }

        match content_type {
            Some(ct) if ct.contains("application/x-www-form-urlencoded") => {
                Self::parse_form(body_str)
            }
            Some(ct) if ct.contains("application/json") => {
                Self::parse_json(body_str)
            }
            Some(ct) if ct.contains("text/plain") => {
                Ok(SafeBody::Text(body_str.to_string()))
            }
            // Default: încearcă form (compatibilitate)
            _ => Self::parse_form(body_str),
        }
    }

    /// Parsează form URL-encoded manual (zero dependințe).
    fn parse_form(body: &str) -> Result<Self, BoundaryError> {
        let mut map = HashMap::new();
        for pair in body.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (key, value) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            };
            let key = url_decode(key);
            let value = url_decode(value);
            map.insert(key, value);
        }
        Ok(SafeBody::Form(map))
    }

    /// Parsează JSON.
    fn parse_json(body: &str) -> Result<Self, BoundaryError> {
        let value: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| BoundaryError::InvalidJson(e.to_string()))?;
        Ok(SafeBody::Json(value))
    }

    /// Returnează o valoare de formular după nume.
    pub fn form_field(&self, name: &str) -> Option<&str> {
        match self {
            SafeBody::Form(map) => map.get(name).map(|s| s.as_str()),
            _ => None,
        }
    }

    /// Returnează TOATE câmpurile de formular.
    pub fn form_fields(&self) -> Option<&HashMap<String, String>> {
        match self {
            SafeBody::Form(map) => Some(map),
            _ => None,
        }
    }

    /// Returnează valoarea JSON.
    pub fn json(&self) -> Option<&serde_json::Value> {
        match self {
            SafeBody::Json(value) => Some(value),
            _ => None,
        }
    }

    /// Returnează textul brut.
    pub fn text(&self) -> Option<&str> {
        match self {
            SafeBody::Text(t) => Some(t.as_str()),
            _ => None,
        }
    }

    /// Verifică dacă body-ul e gol.
    pub fn is_empty(&self) -> bool {
        matches!(self, SafeBody::Empty)
    }
}

/// URL-decode manual: %XX → caracter, + → spațiu
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '+' {
            result.push(' ');
        } else if c == '%' {
            let hi = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
            let lo = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
            result.push((hi as u8 * 16 + lo as u8) as char);
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_body() {
        let b = SafeBody::parse(Some("application/x-www-form-urlencoded"), b"").unwrap();
        assert!(b.is_empty());
    }

    #[test]
    fn test_form_simple() {
        let b = SafeBody::parse(
            Some("application/x-www-form-urlencoded"),
            b"email=test@test.com&qty=3"
        ).unwrap();
        assert_eq!(b.form_field("email"), Some("test@test.com"));
        assert_eq!(b.form_field("qty"), Some("3"));
    }

    #[test]
    fn test_form_url_encoded() {
        let b = SafeBody::parse(
            Some("application/x-www-form-urlencoded"),
            b"name=Ion+Popescu&note=Salut%20lume"
        ).unwrap();
        assert_eq!(b.form_field("name"), Some("Ion Popescu"));
        assert_eq!(b.form_field("note"), Some("Salut lume"));
    }

    #[test]
    fn test_json() {
        let b = SafeBody::parse(
            Some("application/json"),
            br#"{"email":"test@test.com","qty":3}"#
        ).unwrap();
        let json = b.json().unwrap();
        assert_eq!(json["email"], "test@test.com");
        assert_eq!(json["qty"], 3);
    }

    #[test]
    fn test_text() {
        let b = SafeBody::parse(
            Some("text/plain"),
            b"hello world"
        ).unwrap();
        assert_eq!(b.text(), Some("hello world"));
    }

    #[test]
    fn test_body_too_large() {
        let large = vec![b'a'; 3 * 1024 * 1024];
        let r = SafeBody::parse(Some("text/plain"), &large);
        assert!(r.is_err());
        assert!(matches!(r, Err(BoundaryError::BodyTooLarge(_))));
    }

    #[test]
    fn test_invalid_utf8() {
        let r = SafeBody::parse(Some("text/plain"), &[0xFF, 0xFF]);
        assert!(r.is_err());
    }

    #[test]
    fn test_default_content_type_form() {
        let b = SafeBody::parse(None, b"a=1&b=2").unwrap();
        assert_eq!(b.form_field("a"), Some("1"));
    }

    #[test]
    fn test_form_fields_all() {
        let b = SafeBody::parse(
            Some("application/x-www-form-urlencoded"),
            b"x=10&y=20"
        ).unwrap();
        let fields = b.form_fields().unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields.get("x"), Some(&"10".to_string()));
    }
}
