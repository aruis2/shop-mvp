// =============================================================================
// 💰 Price — Preț în bani (1/100 dintr-un leu)
// =============================================================================
// GARANTAT: strict pozitiv, maximum 10.000 lei, fără floating point errors
// =============================================================================

use crate::types::error::InputError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(i32);

impl Price {
    /// Creează un preț din bani. Validare:
    /// - Strict pozitiv (> 0)
    /// - Maximum 10.000 lei (1.000.000 bani)
    pub fn new(bani: i32) -> Result<Self, InputError> {
        if bani <= 0 {
            return Err(InputError::InvalidPrice("Prețul trebuie să fie strict pozitiv".into()));
        }
        if bani > 1_000_000 {
            return Err(InputError::InvalidPrice("Prețul maxim e 10.000 lei".into()));
        }
        Ok(Price(bani))
    }

    /// Calculează totalul pentru o cantitate, cu verificare de overflow
    pub fn total(qty: u32, unit: Price) -> Result<Self, InputError> {
        let total = (qty as i64) * (unit.0 as i64);
        if total > i32::MAX as i64 {
            return Err(InputError::Overflow("Preț total depășește limita".into()));
        }
        if total <= 0 {
            return Err(InputError::InvalidPrice("Totalul trebuie să fie pozitiv".into()));
        }
        Ok(Price(total as i32))
    }

    pub fn as_bani(&self) -> i32 { self.0 }
    pub fn as_lei(&self) -> f64 { self.0 as f64 / 100.0 }
    pub fn as_lei_str(&self) -> String { format!("{:.2}", self.as_lei()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_rejects_zero() { assert!(Price::new(0).is_err()); }
    #[test]
    fn price_rejects_negative() { assert!(Price::new(-1).is_err()); }
    #[test]
    fn price_accepts_valid() { assert!(Price::new(100).is_ok()); }
    #[test]
    fn price_rejects_too_large() { assert!(Price::new(1_000_001).is_err()); }
    #[test]
    fn price_total_overflow_detected() {
        assert!(Price::total(100_000, Price::new(100_000).unwrap()).is_err());
    }
    #[test]
    fn price_to_lei_string() {
        assert_eq!(Price::new(24999).unwrap().as_lei_str(), "249.99");
    }
}
