// =============================================================================
// 💱 Currency — Valută suportată
// =============================================================================

use crate::types::error::InputError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Ron,
    Usd,
    Eur,
}

impl Currency {
    pub fn parse(s: &str) -> Result<Self, InputError> {
        match s.trim().to_lowercase().as_str() {
            "ron" | "lei" | "rol" => Ok(Currency::Ron),
            "usd" | "$" => Ok(Currency::Usd),
            "eur" | "euro" | "€" => Ok(Currency::Eur),
            _ => Err(InputError::InvalidCurrency(format!(
                "Valută nesusținută: '{s}'. Acceptate: ron, usd, eur"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::Ron => "ron",
            Currency::Usd => "usd",
            Currency::Eur => "eur",
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str().to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_ron() { assert_eq!(Currency::parse("ron").unwrap(), Currency::Ron); }
    #[test]
    fn parse_lei() { assert_eq!(Currency::parse("lei").unwrap(), Currency::Ron); }
    #[test]
    fn parse_usd() { assert_eq!(Currency::parse("usd").unwrap(), Currency::Usd); }
    #[test]
    fn parse_eur() { assert_eq!(Currency::parse("eur").unwrap(), Currency::Eur); }
    #[test]
    fn parse_rejects_unknown() { assert!(Currency::parse("gbp").is_err()); }
}
