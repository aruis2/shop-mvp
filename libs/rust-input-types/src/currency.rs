// =============================================================================
// 💵 Currency — Monedă garantat validă (RON, USD, EUR)
// =============================================================================

use crate::error::InputError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Ron,
    Usd,
    Eur,
}

impl Currency {
    pub fn parse(s: &str) -> Result<Self, InputError> {
        match s.to_uppercase().as_str() {
            "RON" => Ok(Currency::Ron),
            "USD" => Ok(Currency::Usd),
            "EUR" => Ok(Currency::Eur),
            _ => Err(InputError::InvalidCurrency(format!("Valută nerecunoscută: {s}"))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::Ron => "RON",
            Currency::Usd => "USD",
            Currency::Eur => "EUR",
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_ron() { assert!(Currency::parse("ron").is_ok()); }
    #[test]
    fn currency_usd() { assert!(Currency::parse("USD").is_ok()); }
    #[test]
    fn currency_eur() { assert!(Currency::parse("eur").is_ok()); }
    #[test]
    fn currency_invalid() { assert!(Currency::parse("gbp").is_err()); }
    #[test]
    fn currency_display() { assert_eq!(Currency::Ron.to_string(), "RON"); }
}
