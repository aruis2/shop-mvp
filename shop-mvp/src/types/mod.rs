// =============================================================================
// 🏭 types/ — Re-export layer pentru rust-input-types + output + extractor
// =============================================================================
// După extracția în rust-input-types crate, acest modul re-exportă tipurile
// din crate și păstrează output.rs (OutputFactory) și extractor.rs (ValidatedForm)
// care sînt cuplate cu shop-mvp (tera, axum, SafeResponse).
// =============================================================================

// Module locale (păstrate în shop-mvp)
pub mod extractor;
pub mod output;

// Re-exporturi din rust-input-types crate
pub use rust_input_types::{
    InputFactory, InputError, QueryValidator,
    Email, Price, Quantity, PhoneNumber, Slug,
    Currency,
    SessionId, UserId, OrderId, ProductId, CategoryId,
    ShippingName, ShippingAddress, Notes, Brand, ProductName, SearchQuery,
    parse_form, get_field, parse_any_into, parse_form_into, FormField,
};
