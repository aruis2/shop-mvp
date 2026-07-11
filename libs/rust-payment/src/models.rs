use serde::{Deserialize, Serialize};

/// Request pentru creare sesiune checkout Stripe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckoutRequest {
    /// ID-ul comenzii în sistemul nostru
    pub order_id: String,
    /// Suma în bani (cents)
    pub amount_bani: i64,
    /// Moneda (ex: "ron", "usd")
    pub currency: String,
    /// URL după plată reușită
    pub success_url: String,
    /// URL după anulare
    pub cancel_url: String,
}

/// Răspuns după crearea sesiunii
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutResponse {
    /// URL-ul Stripe Checkout
    pub checkout_url: String,
    /// ID-ul sesiunii Stripe
    pub session_id: String,
}
