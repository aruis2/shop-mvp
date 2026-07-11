// =============================================================================
// 🔗 Slug — URL-friendly string
// =============================================================================
// GARANTAT: litere mici, cifre, cratime. Fără spații, diacritice, caractere speciale.
// =============================================================================

use crate::types::error::InputError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slug(String);

impl Slug {
    pub fn parse(s: &str) -> Result<Self, InputError> {
        let s = s.trim().to_lowercase();
        if s.is_empty() {
            return Err(InputError::InvalidSlug("Slug-ul e gol".into()));
        }
        if s.len() > 200 {
            return Err(InputError::InvalidSlug("Slug-ul e prea lung (max 200)".into()));
        }
        if !s.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(InputError::InvalidSlug(
                "Slug-ul poate conține doar litere, cifre și cratime".into(),
            ));
        }
        if s.starts_with('-') || s.ends_with('-') {
            return Err(InputError::InvalidSlug("Slug-ul nu poate începe sau termina cu '-'".into()));
        }
        Ok(Slug(s))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for Slug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty() { assert!(Slug::parse("").is_err()); }
    #[test]
    fn rejects_spaces() { assert!(Slug::parse("my slug").is_err()); }
    #[test]
    fn rejects_uppercase() { assert_eq!(Slug::parse("My-Slug").unwrap().as_str(), "my-slug"); }
    #[test]
    fn rejects_special_chars() { assert!(Slug::parse("product@123").is_err()); }
    #[test]
    fn accepts_valid() { assert!(Slug::parse("telefon-samsung-galaxy").is_ok()); }
}
