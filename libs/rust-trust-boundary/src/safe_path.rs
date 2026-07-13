//! # SafePath — URL path sigur (fără path traversal, fără caractere periculoase)
//!
//! ## Principiu
//! Orice path primit de la browser e POTENȚIAL PERICULOS.
//! SafePath normalizează și validează înainte să ajungă la orice handler.
//!
//! ## Protecții
//! - Blochează `..` (path traversal) ← OWASP ASVS V5.1.3
//! - Blochează caractere nule și control
//! - Normalizează slash-urile duble
//! - Limitează lungimea (255 caractere)

use crate::BoundaryError;

/// Un path URL garantat sigur.
///
/// # Garanții
/// - Fără `..` sau `../` (path traversal)
/// - Fără caractere nule (`\0`)
/// - Fără caractere de control (0x00-0x1F)
/// - Lungime maximă 255
/// - Începe cu `/`
#[derive(Debug, Clone, PartialEq)]
pub struct SafePath(String);

impl SafePath {
    /// Parsează și validează un path URL.
    ///
    /// # Erori
    /// - `PathTraversalDetected` — conține `..`
    /// - `InvalidCharacters` — conține null sau control chars
    /// - `PathTooLong` — depășește 255 caractere
    /// - `InvalidPath` — nu începe cu `/`
    pub fn parse(path: &str) -> Result<Self, BoundaryError> {
        // 1. Verifică lungimea
        if path.len() > 255 {
            return Err(BoundaryError::PathTooLong(path.len()));
        }

        // 2. Trebuie să înceapă cu /
        if !path.starts_with('/') {
            return Err(BoundaryError::InvalidPath(path.to_string()));
        }

        // 3. Verifică caractere periculoase
        for c in path.chars() {
            match c {
                '\0' | '\x01'..='\x1f' => {
                    return Err(BoundaryError::InvalidCharacters(path.to_string()));
                }
                _ => {}
            }
        }

        // 4. Blochează path traversal
        if path.contains("..") {
            // Verifică dacă e un `..` real (nu face parte dintr-un cuvânt)
            let segments: Vec<&str> = path.split('/').collect();
            for seg in &segments {
                if *seg == ".." {
                    return Err(BoundaryError::PathTraversalDetected(path.to_string()));
                }
            }
        }

        // 5. Normalizează slash-urile duble
        let normalized = normalize_slashes(path);

        Ok(SafePath(normalized))
    }

    /// Returnează path-ul ca string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returnează segmentele path-ului.
    pub fn segments(&self) -> Vec<&str> {
        self.0.split('/').filter(|s| !s.is_empty()).collect()
    }

    /// Verifică dacă path-ul începe cu un prefix dat.
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }
}

impl std::fmt::Display for SafePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Normalizează slash-urile duble: `//` → `/`
fn normalize_slashes(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut prev_was_slash = false;
    for c in path.chars() {
        if c == '/' {
            if prev_was_slash {
                continue; // skip dublură
            }
            prev_was_slash = true;
        } else {
            prev_was_slash = false;
        }
        result.push(c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_path() {
        let p = SafePath::parse("/products").unwrap();
        assert_eq!(p.as_str(), "/products");
    }

    #[test]
    fn test_root() {
        let p = SafePath::parse("/").unwrap();
        assert_eq!(p.as_str(), "/");
    }

    #[test]
    fn test_path_with_segments() {
        let p = SafePath::parse("/product/laptop-dell").unwrap();
        assert_eq!(p.segments(), vec!["product", "laptop-dell"]);
    }

    #[test]
    fn test_path_traversal_blocked() {
        let r = SafePath::parse("/../etc/passwd");
        assert!(r.is_err());
        assert!(matches!(r, Err(BoundaryError::PathTraversalDetected(_))));
    }

    #[test]
    fn test_path_traversal_middle() {
        let r = SafePath::parse("/products/../admin");
        assert!(r.is_err());
    }

    #[test]
    fn test_double_slash_normalized() {
        let p = SafePath::parse("/products//detail").unwrap();
        assert_eq!(p.as_str(), "/products/detail");
    }

    #[test]
    fn test_null_byte_blocked() {
        let r = SafePath::parse("/products\0.html");
        assert!(r.is_err());
    }

    #[test]
    fn test_too_long() {
        let long = "/".to_string() + &"a".repeat(256);
        let r = SafePath::parse(&long);
        assert!(r.is_err());
    }

    #[test]
    fn test_no_leading_slash() {
        let r = SafePath::parse("products");
        assert!(r.is_err());
    }

    #[test]
    fn test_path_with_query_string_whole() {
        // SafePath parsează doar path-ul, nu query
        let p = SafePath::parse("/search").unwrap();
        assert_eq!(p.as_str(), "/search");
    }

    #[test]
    fn test_starts_with() {
        let p = SafePath::parse("/admin/products").unwrap();
        assert!(p.starts_with("/admin"));
        assert!(!p.starts_with("/cart"));
    }
}
