// =============================================================================
// 🔢 Quantity — Cantitate validă
// =============================================================================
// GARANTAT: 1..999
// =============================================================================

use crate::types::error::InputError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantity(u32);

impl Quantity {
    pub fn new(val: i32) -> Result<Self, InputError> {
        if val < 1 { return Err(InputError::InvalidQuantity("Cantitatea minimă e 1".into())); }
        if val > 999 { return Err(InputError::InvalidQuantity("Cantitatea maximă e 999".into())); }
        Ok(Quantity(val as u32))
    }

    pub fn get(&self) -> u32 { self.0 }
}
