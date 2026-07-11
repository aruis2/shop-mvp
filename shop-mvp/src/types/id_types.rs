// =============================================================================
// 🆔 ID-uri tipate — SessionId, UserId, OrderId, ProductId, CategoryId
// =============================================================================
// Fiecare ID e un tip SEPARAT — nu poți pasa un OrderId unde se așteaptă UserId
// SINGURA cale de a crea un ID din input extern: `::parse()`
// =============================================================================

use uuid::Uuid;
use crate::types::error::InputError;

macro_rules! make_id_type {
    ($name:ident, $err:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            /// Creează un ID nou (server-side generation — OK)
            pub fn new() -> Self { $name(Uuid::new_v4()) }

            /// Parsează un string în ID (user input — singura cale permisă)
            pub fn parse(s: &str) -> Result<Self, InputError> {
                Uuid::parse_str(s).map($name).map_err(|_| $err)
            }

            pub fn as_uuid(&self) -> &Uuid { &self.0 }
            pub fn to_string(&self) -> String { self.0.to_string() }
        }
    };
}

make_id_type!(SessionId, InputError::InvalidSessionId);
make_id_type!(UserId, InputError::InvalidUserId);
make_id_type!(OrderId, InputError::InvalidOrderId);

// --- ID-uri numerice (pentru PostgreSQL SERIAL) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductId(i32);

impl ProductId {
    pub fn new(val: i32) -> Result<Self, InputError> {
        if val <= 0 { return Err(InputError::InvalidProductId); }
        Ok(ProductId(val))
    }
    pub fn get(&self) -> i32 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryId(i32);

impl CategoryId {
    pub fn new(val: i32) -> Result<Self, InputError> {
        if val <= 0 { return Err(InputError::InvalidCategoryId); }
        Ok(CategoryId(val))
    }
    pub fn get(&self) -> i32 { self.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_roundtrip() {
        let sid = SessionId::new();
        let s = sid.to_string();
        assert!(SessionId::parse(&s).is_ok());
    }

    #[test]
    fn session_id_rejects_invalid() {
        assert!(SessionId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn product_id_positive() {
        assert!(ProductId::new(1).is_ok());
        assert!(ProductId::new(0).is_err());
        assert!(ProductId::new(-1).is_err());
    }

    #[test]
    fn user_id_vs_order_id_different_types() {
        // ❌ NU compilează: UserId vs OrderId
        // let u = UserId::new();
        // let o: OrderId = u;
        // error: expected `OrderId`, found `UserId`
    }
}
