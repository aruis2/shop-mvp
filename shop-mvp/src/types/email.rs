// =============================================================================
// 📧 Email — Garantat valid
// =============================================================================

use serde::Serialize;
use crate::types::error::InputError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Email(String);

impl Email {
    /// Parsează un string în Email. GARANTAT valid după parse.
    pub fn parse(s: &str) -> Result<Self, InputError> {
        let s = s.trim().to_lowercase();
        if s.is_empty() { return Err(InputError::EmptyEmail); }
        if !s.contains('@') { return Err(InputError::InvalidEmail("Lipsă @".into())); }
        let parts: Vec<&str> = s.splitn(2, '@').collect();
        let local = parts[0];
        let domain = parts[1];
        if local.is_empty() { return Err(InputError::InvalidEmail("Partea locală e goală".into())); }
        if domain.is_empty() { return Err(InputError::InvalidEmail("Domeniul e gol".into())); }
        if !domain.contains('.') { return Err(InputError::InvalidEmail("Domeniul trebuie să conțină un punct".into())); }
        if s.len() > 254 { return Err(InputError::InvalidEmail("Prea lung (max 254 caractere)".into())); }
        Ok(Email(s))
    }

    pub fn as_str(&self) -> &str { &self.0 }
    pub fn domain(&self) -> &str { self.0.split('@').nth(1).unwrap_or("") }
    pub fn local(&self) -> &str { self.0.split('@').next().unwrap_or("") }
}

// Permite folosirea lui Email în Tera templates
impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() { assert!(Email::parse("").is_err()); }
    #[test]
    fn rejects_no_at() { assert!(Email::parse("user").is_err()); }
    #[test]
    fn rejects_no_domain() { assert!(Email::parse("user@").is_err()); }
    #[test]
    fn rejects_no_tld() { assert!(Email::parse("user@domain").is_err()); }
    #[test]
    fn accepts_valid() { assert!(Email::parse("user@domain.com").is_ok()); }
    #[test]
    fn trims_whitespace() { assert!(Email::parse("  user@domain.com  ").is_ok()); }
    #[test]
    fn lowercases() { assert_eq!(Email::parse("USER@Domain.COM").unwrap().as_str(), "user@domain.com"); }
}
