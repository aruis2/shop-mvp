// =============================================================================
// 🏭 rust-input-types — Parse, Don't Validate (PHILOSOPHY #6, #13)
// =============================================================================
// ⚠️  ABSOLUT TOT ce introduce utilizatorul trece PRIN AICI.
//      Niciun cod din interior nu are voie să folosească date
//      care n-au trecut prin această uzină (conveior).
// =============================================================================
// Reguli:
// 1. Orice `String`, `i32`, `Uuid` din request → trece prin `InputFactory::parse_*()`
// 2. Tipurile returnate (Email, Price, etc.) au constructorii PRIVAȚI
// 3. SINGURA cale de a crea un tip valid e prin `::parse()` / `::new()` cu validare
// 4. Orice handler care primește un tip de aici știe că e GARANTAT valid
//    (Parse, Don't Validate — Alexis King / Yaron Minsky, Jane Street)
// =============================================================================

pub mod currency;
pub mod email;
pub mod error;
pub mod id_types;
pub mod parser;
pub mod phone;
pub mod price;
pub mod quantity;
pub mod slug;
pub mod text;

// Re-exporturi pentru acces direct (rust_input_types::Email vs rust_input_types::email::Email)
pub use currency::Currency;
pub use email::Email;
pub use error::InputError;
pub use id_types::*;
pub use phone::PhoneNumber;
pub use price::Price;
pub use quantity::Quantity;
pub use slug::Slug;
pub use text::*;
pub use parser::{parse_form, get_field, parse_any_into, parse_form_into, FormField};

// =============================================================================
// 🏭 InputFactory — CONVEIORUL. SINGURUL punct de intrare pentru date externe.
// =============================================================================
// Orice handler care primește date de la utilizator:
//   1. String-urile → InputFactory::parse_*()
//   2. Tipurile returnate sînt GARANTAT valide
//   3. Nu mai verifici NICIODATĂ
// =============================================================================
pub struct InputFactory;

impl InputFactory {
    pub fn parse_email(s: &str) -> Result<Email, InputError> {
        Email::parse(s)
    }
    pub fn parse_price(bani: i32) -> Result<Price, InputError> {
        Price::new(bani)
    }
    pub fn parse_qty(val: i32) -> Result<Quantity, InputError> {
        Quantity::new(val)
    }
    pub fn parse_phone(s: &str) -> Result<PhoneNumber, InputError> {
        PhoneNumber::parse(s)
    }
    pub fn parse_slug(s: &str) -> Result<Slug, InputError> {
        Slug::parse(s)
    }
    pub fn parse_session_id(s: &str) -> Result<SessionId, InputError> {
        SessionId::parse(s)
    }
    pub fn parse_user_id(s: &str) -> Result<UserId, InputError> {
        UserId::parse(s)
    }
    pub fn parse_order_id(s: &str) -> Result<OrderId, InputError> {
        OrderId::parse(s)
    }
    pub fn parse_product_id(val: i32) -> Result<ProductId, InputError> {
        ProductId::new(val)
    }
    pub fn parse_category_id(val: i32) -> Result<CategoryId, InputError> {
        CategoryId::new(val)
    }
    pub fn parse_name(s: &str) -> Result<ShippingName, InputError> {
        ShippingName::parse(s)
    }
    pub fn parse_address(s: &str) -> Result<ShippingAddress, InputError> {
        ShippingAddress::parse(s)
    }
    pub fn parse_notes(s: &str) -> Result<Notes, InputError> {
        Notes::parse(s)
    }
    pub fn parse_brand(s: &str) -> Result<Brand, InputError> {
        Brand::parse(s)
    }
    pub fn parse_product_name(s: &str) -> Result<ProductName, InputError> {
        ProductName::parse(s)
    }
    pub fn parse_search(s: &str) -> Result<SearchQuery, InputError> {
        SearchQuery::parse(s)
    }
    pub fn parse_currency(s: &str) -> Result<Currency, InputError> {
        Currency::parse(s)
    }
}

// =============================================================================
// 🔍 QueryValidator — validează și LOGHEAZĂ query params invalide
// =============================================================================
// Problema: serde ignoră tăcut valorile invalide în Option<T>.
// Ex: ?page=abc → page=None, handlerul nu știe, folosește default.
// QueryValidator prinde asta: loghează și returnează o valoare sigură.
// =============================================================================
pub struct QueryValidator;

impl QueryValidator {
    /// Validează `page`. Loghează dacă e invalid.
    pub fn page(val: Option<i64>, default: i64) -> i64 {
        match val {
            Some(p) if p >= 1 => p,
            Some(p) => {
                tracing::warn!(target: "query", "page invalid: {} (folosesc default {})", p, default);
                default
            }
            None => default,
        }
    }

    /// Validează un UUID string. Loghează dacă e invalid.
    pub fn uuid(val: Option<String>, name: &str) -> Option<uuid::Uuid> {
        match val {
            Some(s) => match uuid::Uuid::parse_str(&s) {
                Ok(id) => Some(id),
                Err(_) => {
                    tracing::warn!(target: "query", "{} UUID invalid: {} (ignorat)", name, s);
                    None
                }
            },
            None => None,
        }
    }

    /// Validează un token JWT (doar format, nu semnătura).
    pub fn token(val: Option<String>, name: &str) -> Option<String> {
        match val {
            Some(t) if t.split('.').count() == 3 => Some(t),
            Some(t) => {
                tracing::warn!(target: "query", "{} token invalid format: {} (ignorat)", name, t);
                None
            }
            None => None,
        }
    }

    /// Validează session_id: nu acceptăm valori foarte lungi sau suspecte.
    pub fn session_id(val: Option<String>, name: &str) -> Option<String> {
        match val {
            Some(s) if s.len() > 256 => {
                tracing::warn!(target: "query", "{} session_id suspect: {} caractere (ignorat)", name, s.len());
                None
            }
            Some(s) => Some(s),
            None => None,
        }
    }

    /// Validează o valoare de header (lungime maximă, fără control chars).
    pub fn header(val: Option<String>, name: &str, max_len: usize) -> Option<String> {
        match val {
            Some(s) if s.len() > max_len => {
                tracing::warn!(target: "header", "{} prea lung: {} caractere (ignorat)", name, s.len());
                None
            }
            Some(s) if s.chars().any(|c| c.is_control() && c != '\t') => {
                tracing::warn!(target: "header", "{} conține control chars (ignorat)", name);
                None
            }
            Some(s) => Some(s),
            None => None,
        }
    }
}
