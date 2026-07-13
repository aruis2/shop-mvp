// =============================================================================
// 🔢 Quantity — Cantitate garantat validă (1..=999)
// =============================================================================

use crate::error::InputError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantity(u32);

impl Quantity {
    pub fn new(val: i32) -> Result<Self, InputError> {
        if val <= 0 {
            return Err(InputError::InvalidQuantity("Cantitatea trebuie să fie strict pozitivă".into()));
        }
        if val > 999 {
            return Err(InputError::InvalidQuantity("Cantitatea maximă e 999".into()));
        }
        Ok(Quantity(val as u32))
    }

    pub fn get(&self) -> u32 { self.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qty_rejects_zero() { assert!(Quantity::new(0).is_err()); }
    #[test]
    fn qty_rejects_negative() { assert!(Quantity::new(-1).is_err()); }
    #[test]
    fn qty_accepts_valid() { assert!(Quantity::new(5).is_ok()); }
    #[test]
    fn qty_rejects_too_large() { assert!(Quantity::new(1000).is_err()); }
}
