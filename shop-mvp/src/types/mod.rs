// =============================================================================
// 🏭 Uzina de Input — Parse, Don't Validate
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
// =============================================================================

pub mod currency;
pub mod email;
pub mod error;
pub mod id_types;
pub mod phone;
pub mod price;
pub mod quantity;
pub mod slug;
pub mod text;

use email::Email;
use error::InputError;
use id_types::*;
use phone::PhoneNumber;
use price::Price;
use quantity::Quantity;
use slug::Slug;
use text::*;

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
    pub fn parse_currency(s: &str) -> Result<currency::Currency, InputError> {
        currency::Currency::parse(s)
    }
}

use email::Email;
use error::InputError;
use id_types::*;
use phone::PhoneNumber;
use price::Price;
use quantity::Quantity;
use slug::Slug;
use text::*;

/// 🏭 InputFactory — SINGURUL punct de intrare pentru date utilizator.
///
/// Toate datele brute (String, i32, etc.) trec prin factory methods.
/// Orice iese din InputFactory e GARANTAT valid — nu mai verifici niciodată.
pub struct InputFactory;

impl InputFactory {
    // ─── Email ────────────────────────────────────────────
    pub fn parse_email(s: &str) -> Result<Email, InputError> {
        Email::parse(s)
    }

    // ─── Preț ─────────────────────────────────────────────
    pub fn parse_price(bani: i32) -> Result<Price, InputError> {
        Price::new(bani)
    }

    // ─── Cantitate ────────────────────────────────────────
    pub fn parse_qty(val: i32) -> Result<Quantity, InputError> {
        Quantity::new(val)
    }

    // ─── Telefon ──────────────────────────────────────────
    pub fn parse_phone(s: &str) -> Result<PhoneNumber, InputError> {
        PhoneNumber::parse(s)
    }

    // ─── Slug ─────────────────────────────────────────────
    pub fn parse_slug(s: &str) -> Result<Slug, InputError> {
        Slug::parse(s)
    }

    // ─── ID-uri ───────────────────────────────────────────
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

    // ─── Text ─────────────────────────────────────────────
    pub fn parse_name(s: &str) -> Result<ShippingName, InputError> {
        ShippingName::parse(s)
    }

    pub fn parse_address(s: &str) -> Result<ShippingAddress, InputError> {
        ShippingAddress::parse(s)
    }

    pub fn parse_notes(s: &str) -> Result<Notes, InputError> {
        Notes::parse(s)
    }

    // ─── Valută ───────────────────────────────────────────
    pub fn parse_currency(s: &str) -> Result<currency::Currency, InputError> {
        currency::Currency::parse(s)
    }

    // ─── Convenience: parse form body ─────────────────────
    // Parsează un cîmp din form după nume, cu eroare customizabilă
    pub fn parse_form_field<'a>(
        body: &'a str,
        field: &str,
    ) -> Result<&'a str, InputError> {
        for pair in body.split('&') {
            if let Some((key, val)) = pair.split_once('=') {
                if key == field {
                    let decoded = urlencoding_decode(val);
                    return Ok(decoded); // lifetime legat de body
                }
            }
        }
        Err(InputError::MissingField(field.to_string()))
    }
}

/// Decodare URL simplă (fără dependencies)
fn urlencoding_decode(s: &str) -> &str {
    s // TODO: implementare full cînd migrăm la parse înainte de handler
}
