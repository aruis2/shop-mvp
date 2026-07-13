// =============================================================================
// ❌ InputError — Toate erorile de parsare
// =============================================================================

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum InputError {
    // Email
    EmptyEmail,
    InvalidEmail(String),
    // Price
    InvalidPrice(String),
    // Quantity
    InvalidQuantity(String),
    // Phone
    InvalidPhone(String),
    // Slug
    InvalidSlug(String),
    // ID-uri
    InvalidSessionId,
    InvalidUserId,
    InvalidOrderId,
    InvalidProductId,
    InvalidCategoryId,
    // Text
    EmptyName,
    NameTooLong,
    EmptyAddress,
    AddressTooLong,
    NotesTooLong,
    // Password
    PasswordTooShort,
    PasswordTooLong,
    PasswordNoUppercase,
    PasswordNoLowercase,
    PasswordNoDigit,
    // URL
    InvalidUrl(String),
    // Token
    InvalidToken(String),
    // Status
    InvalidOrderStatus(String),
    InvalidPaymentStatus(String),
    // Currency
    InvalidCurrency(String),
    // Search
    SearchQueryTooLong,
    // Brand
    EmptyBrand,
    BrandTooLong,
    // Product name
    EmptyProductName,
    ProductNameTooLong,
    // Cîmp lipsă
    MissingField(String),
    // Overflow
    Overflow(String),
    // Generic
    Custom(String),
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputError::EmptyEmail => write!(f, "Email-ul nu poate fi gol"),
            InputError::InvalidEmail(msg) => write!(f, "Email invalid: {msg}"),
            InputError::InvalidPrice(msg) => write!(f, "Preț invalid: {msg}"),
            InputError::InvalidQuantity(msg) => write!(f, "Cantitate invalidă: {msg}"),
            InputError::InvalidPhone(msg) => write!(f, "Telefon invalid: {msg}"),
            InputError::InvalidSlug(msg) => write!(f, "Slug invalid: {msg}"),
            InputError::InvalidSessionId => write!(f, "Session ID invalid"),
            InputError::InvalidUserId => write!(f, "User ID invalid"),
            InputError::InvalidOrderId => write!(f, "Order ID invalid"),
            InputError::InvalidProductId => write!(f, "Product ID invalid"),
            InputError::InvalidCategoryId => write!(f, "Category ID invalid"),
            InputError::EmptyName => write!(f, "Numele nu poate fi gol"),
            InputError::NameTooLong => write!(f, "Numele e prea lung (max 200)"),
            InputError::EmptyAddress => write!(f, "Adresa nu poate fi goală"),
            InputError::AddressTooLong => write!(f, "Adresa e prea lungă (max 500)"),
            InputError::NotesTooLong => write!(f, "Notele sînt prea lungi (max 2000)"),
            InputError::PasswordTooShort => write!(f, "Parola trebuie să aibă minim 8 caractere"),
            InputError::PasswordTooLong => write!(f, "Parola e prea lungă (max 128)"),
            InputError::PasswordNoUppercase => write!(f, "Parola trebuie să conțină o literă mare"),
            InputError::PasswordNoLowercase => write!(f, "Parola trebuie să conțină o literă mică"),
            InputError::PasswordNoDigit => write!(f, "Parola trebuie să conțină o cifră"),
            InputError::InvalidUrl(msg) => write!(f, "URL invalid: {msg}"),
            InputError::InvalidToken(msg) => write!(f, "Token invalid: {msg}"),
            InputError::InvalidOrderStatus(msg) => write!(f, "Status comandă invalid: {msg}"),
            InputError::InvalidPaymentStatus(msg) => write!(f, "Status plată invalid: {msg}"),
            InputError::InvalidCurrency(msg) => write!(f, "Valută invalidă: {msg}"),
            InputError::SearchQueryTooLong => write!(f, "Căutarea e prea lungă (max 200)"),
            InputError::EmptyBrand => write!(f, "Brandul nu poate fi gol"),
            InputError::BrandTooLong => write!(f, "Brandul e prea lung (max 100)"),
            InputError::EmptyProductName => write!(f, "Numele produsului nu poate fi gol"),
            InputError::ProductNameTooLong => write!(f, "Numele produsului e prea lung (max 200)"),
            InputError::MissingField(field) => write!(f, "Cîmpul '{field}' lipsește"),
            InputError::Overflow(msg) => write!(f, "Overflow: {msg}"),
            InputError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for InputError {}
