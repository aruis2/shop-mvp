// =============================================================================
// 📞 PhoneNumber — Telefon garantat valid (10 cifre, prefix România)
// =============================================================================

use crate::error::InputError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneNumber(String);

impl PhoneNumber {
    pub fn parse(s: &str) -> Result<Self, InputError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(InputError::InvalidPhone("Telefonul nu poate fi gol".into()));
        }
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 10 {
            return Err(InputError::InvalidPhone(
                format!("Telefonul trebuie să aibă 10 cifre (are {})", digits.len())
            ));
        }
        if !digits.starts_with('0') {
            return Err(InputError::InvalidPhone("Telefonul trebuie să înceapă cu 0".into()));
        }
        Ok(PhoneNumber(digits))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for PhoneNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_rejects_empty() { assert!(PhoneNumber::parse("").is_err()); }
    #[test]
    fn phone_rejects_short() { assert!(PhoneNumber::parse("0722").is_err()); }
    #[test]
    fn phone_rejects_no_prefix() { assert!(PhoneNumber::parse("722123456").is_err()); }
    #[test]
    fn phone_accepts_valid() { assert!(PhoneNumber::parse("0722123456").is_ok()); }
    #[test]
    fn phone_strips_nondigits() { assert!(PhoneNumber::parse("0712 345 678").is_ok()); }
}
