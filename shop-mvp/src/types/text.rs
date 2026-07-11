// =============================================================================
// 📝 Text types — ShippingName, ShippingAddress, Notes, Brand, ProductName, SearchQuery
// =============================================================================

use crate::types::error::InputError;

macro_rules! make_text_type {
    ($name:ident, $empty_err:expr, $long_err:expr, $max_len:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(String);

        impl $name {
            pub fn parse(s: &str) -> Result<Self, InputError> {
                let s = s.trim().to_string();
                if s.is_empty() { return Err($empty_err); }
                if s.len() > $max_len { return Err($long_err); }
                Ok($name(s))
            }
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

make_text_type!(ShippingName, InputError::EmptyName, InputError::NameTooLong, 200);
make_text_type!(ShippingAddress, InputError::EmptyAddress, InputError::AddressTooLong, 500);
make_text_type!(Notes, InputError::Custom("Note empty".into()), InputError::NotesTooLong, 2000);
make_text_type!(Brand, InputError::EmptyBrand, InputError::BrandTooLong, 100);
make_text_type!(ProductName, InputError::EmptyProductName, InputError::ProductNameTooLong, 200);
make_text_type!(SearchQuery, InputError::Custom("Căutare goală".into()), InputError::SearchQueryTooLong, 200);
