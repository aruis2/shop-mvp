//! # rust-payment
//!
//! Modul LEGO pentru plăți. Suportă Stripe.
//!
//! ## Teste
//! ```bash
//! cargo test -p rust-payment
//! ```

mod models;
mod stripe;
mod mock;
pub mod retry;

pub use models::{CheckoutResponse, CreateCheckoutRequest};
pub use stripe::StripePayment;
pub use mock::MockPayment;
pub use retry::RetryPayment;

use async_trait::async_trait;
use thiserror::Error;

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, Error)]
pub enum PaymentError {
    #[error("Stripe error: {0}")]
    Stripe(String),

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Missing API key")]
    MissingApiKey,

    #[error("Refund failed: {0}")]
    RefundFailed(String),
}

// ============================================================================
// Trait principal — PaymentRepo
// ============================================================================

#[async_trait]
pub trait PaymentRepo: Send + Sync {
    /// Creează o sesiune de checkout
    async fn create_checkout(&self, req: CreateCheckoutRequest) -> Result<CheckoutResponse, PaymentError>;

    /// Rambursează o plată după ID-ul dat de provider (ex: session_id Stripe)
    async fn refund_payment(&self, payment_provider_id: &str) -> Result<(), PaymentError>;
}

// ============================================================================
// Teste
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkout_request() {
        let req = CreateCheckoutRequest {
            order_id: "order-123".into(),
            amount_bani: 10000,
            currency: "ron".into(),
            success_url: "https://example.com/success".into(),
            cancel_url: "https://example.com/cancel".into(),
        };
        assert_eq!(req.amount_bani, 10000);
        assert_eq!(req.currency, "ron");
    }

    #[test]
    fn test_checkout_response() {
        let resp = CheckoutResponse {
            checkout_url: "https://checkout.stripe.com/session_123".into(),
            session_id: "session_123".into(),
        };
        assert!(resp.checkout_url.contains("stripe.com"));
    }

    #[test]
    fn test_payment_errors() {
        let err = PaymentError::InvalidAmount;
        assert_eq!(format!("{}", err), "Invalid amount");
    }
}
