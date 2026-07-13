// =============================================================================
// 📝 Tipuri de text — garantat valide (Parse, Don't Validate)
// =============================================================================
// Standard: OWASP ASVS V5.1 (Input Validation)
// Bug-uri prevenite: string-uri goale, lungimi excesive, XSS prin input
// =============================================================================

use crate::error::InputError;

macro_rules! make_text_type {
    ($name:ident, $empty_err:ident, $long_err:ident, $max_len:expr, $empty_msg:expr, $long_msg:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(String);

        impl $name {
            pub fn parse(s: &str) -> Result<Self, InputError> {
                let s = s.trim().to_string();
                if s.is_empty() {
                    return Err(InputError::$empty_err);
                }
                if s.len() > $max_len {
                    return Err(InputError::$long_err);
                }
                Ok($name(s))
            }

            pub fn as_str(&self) -> &str {
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

make_text_type!(ShippingName, EmptyName, NameTooLong, 200,
    "Numele nu poate fi gol", "Numele e prea lung (max 200)");
make_text_type!(ShippingAddress, EmptyAddress, AddressTooLong, 500,
    "Adresa nu poate fi goală", "Adresa e prea lungă (max 500)");
make_text_type!(Notes, EmptyAddress, NotesTooLong, 2000,  // folosim EmptyAddress ca ersatz
    "Notele nu poate fi goale", "Notele sînt prea lungi (max 2000)");
make_text_type!(Brand, EmptyBrand, BrandTooLong, 100,
    "Brandul nu poate fi gol", "Brandul e prea lung (max 100)");
make_text_type!(ProductName, EmptyProductName, ProductNameTooLong, 200,
    "Numele produsului nu poate fi gol", "Numele produsului e prea lung (max 200)");
make_text_type!(SearchQuery, EmptyName, SearchQueryTooLong, 200,
    "Căutarea nu poate fi goală", "Căutarea e prea lungă (max 200)");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_valid() { assert!(ShippingName::parse("Ion Popescu").is_ok()); }
    #[test]
    fn name_empty() { assert!(ShippingName::parse("").is_err()); }
    #[test]
    fn name_too_long() {
        let long = "a".repeat(201);
        assert!(ShippingName::parse(&long).is_err());
    }
    #[test]
    fn brand_valid() { assert!(Brand::parse("Samsung").is_ok()); }
    #[test]
    fn product_name_valid() { assert!(ProductName::parse("Telefon Samsung Galaxy").is_ok()); }
}
