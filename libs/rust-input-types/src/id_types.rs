// =============================================================================
// 🆔 Tipuri de ID-uri — garantat valide (Parse, Don't Validate)
// =============================================================================
// Standard: OWASP ASVS V2.8 (IDOR), V5.1 (Input)
// Filosofie: fiecare ID e un tip separat — compilatorul prinde confuzia între
//            SessionId și UserId (PHILOSOPHY #8 — Newtype pattern)
// Bug-uri prevenite: SessionId în loc de UserId, string în loc de UUID
// =============================================================================

use crate::error::InputError;
use uuid::Uuid;

macro_rules! make_id_type {
    ($name:ident, $parse_err:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                $name(Uuid::new_v4())
            }

            pub fn parse(s: &str) -> Result<Self, InputError> {
                let s = s.trim();
                if s.is_empty() {
                    return Err(InputError::$parse_err);
                }
                let uuid = Uuid::parse_str(s).map_err(|_| InputError::$parse_err)?;
                Ok($name(uuid))
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

make_id_type!(SessionId, InvalidSessionId);
make_id_type!(UserId, InvalidUserId);
make_id_type!(OrderId, InvalidOrderId);

// ID-uri numerice (pentru PostgreSQL SERIAL)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductId(i32);

impl ProductId {
    pub fn new(val: i32) -> Result<Self, InputError> {
        if val <= 0 { return Err(InputError::InvalidProductId); }
        Ok(ProductId(val))
    }

    pub fn parse(s: &str) -> Result<Self, InputError> {
        let s = s.trim();
        if s.is_empty() { return Err(InputError::InvalidProductId); }
        let val: i32 = s.parse().map_err(|_| InputError::InvalidProductId)?;
        if val <= 0 { return Err(InputError::InvalidProductId); }
        Ok(ProductId(val))
    }

    pub fn get(&self) -> i32 { self.0 }
}

impl std::fmt::Display for ProductId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CategoryId(i32);

impl CategoryId {
    pub fn new(val: i32) -> Result<Self, InputError> {
        if val <= 0 { return Err(InputError::InvalidCategoryId); }
        Ok(CategoryId(val))
    }

    pub fn parse(s: &str) -> Result<Self, InputError> {
        let s = s.trim();
        if s.is_empty() { return Err(InputError::InvalidCategoryId); }
        let val: i32 = s.parse().map_err(|_| InputError::InvalidCategoryId)?;
        if val <= 0 { return Err(InputError::InvalidCategoryId); }
        Ok(CategoryId(val))
    }

    pub fn get(&self) -> i32 { self.0 }
}

impl std::fmt::Display for CategoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_parse_valid() {
        let id = SessionId::parse("550e8400-e29b-41d4-a716-446655440000");
        assert!(id.is_ok());
    }

    #[test]
    fn session_id_parse_invalid() {
        assert!(SessionId::parse("nu-e-uuid").is_err());
    }

    #[test]
    fn session_id_empty() {
        assert!(SessionId::parse("").is_err());
    }
}
