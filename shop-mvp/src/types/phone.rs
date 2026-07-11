// =============================================================================
// 📞 PhoneNumber — Număr de telefon românesc
// =============================================================================
// GARANTAT: 10 cifre, începe cu 0
// =============================================================================

use crate::types::error::InputError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneNumber(String);

impl PhoneNumber {
    pub fn parse(s: &str) -> Result<Self, InputError> {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 10 {
            return Err(InputError::InvalidPhone("Telefonul trebuie să aibă 10 cifre".into()));
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
        // Formatare: 0712 345 678
        let s = &self.0;
        write!(f, "{} {} {} {}", &s[0..4], &s[4..7], &s[7..9], &s[9..10])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_10_digits() { assert!(PhoneNumber::parse("0712345678").is_ok()); }
    #[test]
    fn rejects_9_digits() { assert!(PhoneNumber::parse("071234567").is_err()); }
    #[test]
    fn rejects_no_prefix_0() { assert!(PhoneNumber::parse("7123456789").is_err()); }
    #[test]
    fn strips_non_digits() { assert!(PhoneNumber::parse("0712 345 678").is_ok()); }
}
