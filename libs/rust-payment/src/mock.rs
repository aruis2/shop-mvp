use async_trait::async_trait;

use crate::{CheckoutResponse, CreateCheckoutRequest, PaymentError, PaymentRepo};

/// 🔧 MockPayment — plată instant, fără Stripe (dev mode)
///
/// Când `MOCK_PAYMENT=true` în environment, în loc să redirecționeze la Stripe,
/// returnează success_url direct. Checkout handler-ul detectează `mock_` în
/// session_id și marchează comanda ca plătită imediat.
pub struct MockPayment;

impl MockPayment {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PaymentRepo for MockPayment {
    async fn create_checkout(&self, req: CreateCheckoutRequest) -> Result<CheckoutResponse, PaymentError> {
        if req.amount_bani <= 0 {
            return Err(PaymentError::InvalidAmount);
        }

        Ok(CheckoutResponse {
            checkout_url: req.success_url,
            session_id: format!("mock_{}", req.order_id),
        })
    }

    async fn refund_payment(&self, _payment_provider_id: &str) -> Result<(), PaymentError> {
        Ok(())
    }
}
