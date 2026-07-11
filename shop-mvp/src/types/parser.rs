// =============================================================================
// 📦 Parser HTTP — propriul parser URL-encoded (zero dependințe)
// =============================================================================
// FILOSOFIE: PHILOSOPHY #13 (zero dependințe unde putem)
// STANDARD: OWASP ASVS V5.1 (Input Validation)
// =============================================================================
//
// Acesta e PRIMUL contact cu datele utilizatorului.
// Fără serde_urlencoded, fără Axum extractors — doar Rust pur.
// Inputul e transformat DIRECT în tipurile noastre sigure.
// =============================================================================

use crate::types::error::InputError;

/// Un cîmp dintr-un form URL-encoded, deja parsat și URL-decodat.
/// Conține doar ce e strict necesar: numele cîmpului și valoarea ca string.
#[derive(Debug)]
pub struct FormField<'a> {
    pub name: &'a str,
    pub value: String,
}

/// Parsează un body URL-encoded în vector de FormField.
/// Aceasta e PRIMA funcție care atinge datele utilizatorului.
///
/// "email=ion%40test.com&qty=3" →
///   [FormField { name: "email", value: "ion@test.com" },
///    FormField { name: "qty", value: "3" }]
pub fn parse_form(body: &str) -> Vec<FormField<'_>> {
    body.split('&')
        .filter_map(|pair| {
            let (raw_key, raw_val) = pair.split_once('=')?;
            Some(FormField {
                name: raw_key,
                value: url_decode(raw_val),
            })
        })
        .collect()
}

/// Extrage valoarea unui cîmp după nume.
pub fn get_field<'a>(fields: &'a [FormField<'a>], name: &str) -> Result<&'a str, InputError> {
    fields.iter()
        .find(|f| f.name == name)
        .map(|f| f.value.as_str())
        .ok_or_else(|| InputError::MissingField(name.to_string()))
}

/// URL-decode manual: %XX → caracter, + → spațiu
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            // % urmat de mai puțin de 2 hex — lasă literal
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

/// Parsează un form DIRECT în tipurile noastre, printr-un closure.
/// Exemplu:
/// ```rust
/// let (email, qty) = parse_form_into(&body, |fields| {
///     let email = InputFactory::parse_email(get_field(fields, "email")?)?;
///     let qty = InputFactory::parse_qty(get_field(fields, "qty")?.parse().unwrap_or(0))?;
///     Ok((email, qty))
/// })?;
/// ```
pub fn parse_form_into<T>(
    body: &str,
    f: impl FnOnce(&[FormField]) -> Result<T, InputError>,
) -> Result<T, InputError> {
    let fields = parse_form(body);
    f(&fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_form() {
        let fields = parse_form("email=test@test.com&qty=3");
        assert_eq!(fields.len(), 2);
        assert_eq!(get_field(&fields, "email").unwrap(), "test@test.com");
        assert_eq!(get_field(&fields, "qty").unwrap(), "3");
    }

    #[test]
    fn test_url_encoded() {
        let fields = parse_form("email=ion%40test.com&name=Ion+Popa");
        assert_eq!(get_field(&fields, "email").unwrap(), "ion@test.com");
        assert_eq!(get_field(&fields, "name").unwrap(), "Ion Popa");
    }

    #[test]
    fn test_empty_form() {
        let fields = parse_form("");
        assert!(fields.is_empty());
    }

    #[test]
    fn test_missing_field() {
        let fields = parse_form("email=test@test.com");
        assert!(get_field(&fields, "qty").is_err());
    }

    #[test]
    fn test_url_decode_percent_20() {
        assert_eq!(url_decode("hello%20world"), "hello world");
    }

    #[test]
    fn test_url_decode_plus() {
        assert_eq!(url_decode("hello+world"), "hello world");
    }

    #[test]
    fn test_url_decode_special_chars() {
        assert_eq!(url_decode("a%2Fb%3Dc"), "a/b=c");
    }
}
