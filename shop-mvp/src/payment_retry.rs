// =============================================================================
// 💳 RetryPayment — Error boundary pentru Stripe
// =============================================================================
// În spirit seL4: acest wrapper adaugă un "retry domain" în jurul plăților.
// Dacă Stripe e temporar indisponibil, nu crapă imediat — încearcă din nou
// cu backoff exponențial. Timeout-ul limitează cât așteptăm.

use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use rust_payment::*;
use tokio::time::timeout;

/// Wrapper care adaugă retry cu exponential backoff + timeout
pub struct RetryPayment {
    inner: Arc<dyn PaymentRepo>,
    max_retries: u32,
    base_delay_ms: u64,
    request_timeout_ms: u64,
}

impl RetryPayment {
    pub fn new(inner: Arc<dyn PaymentRepo>) -> Self {
        Self {
            inner,
            max_retries: 3,
            base_delay_ms: 200,   // 200ms, 400ms, 800ms
            request_timeout_ms: 10_000, // 10s timeout per request
        }
    }

    /// Configurare personalizată
    #[allow(dead_code)]
    pub fn with_retry(mut self, max: u32, base_ms: u64) -> Self {
        self.max_retries = max;
        self.base_delay_ms = base_ms;
        self
    }

    #[allow(dead_code)]
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.request_timeout_ms = ms;
        self
    }
}

#[async_trait]
impl PaymentRepo for RetryPayment {
    async fn create_checkout(&self, req: CreateCheckoutRequest) -> Result<CheckoutResponse, PaymentError> {
        let mut last_err = None;
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = self.base_delay_ms * (1u64 << (attempt - 1)); // exponential: 200, 400, 800
                tracing::warn!("🔄 Retry create_checkout (attempt {}/{}) după {}ms", attempt, self.max_retries, delay);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            let cloned = CreateCheckoutRequest {
                order_id: req.order_id.clone(),
                amount_bani: req.amount_bani,
                currency: req.currency.clone(),
                success_url: req.success_url.clone(),
                cancel_url: req.cancel_url.clone(),
            };
            match timeout(Duration::from_millis(self.request_timeout_ms), self.inner.create_checkout(cloned)).await {
                Ok(Ok(resp)) => return Ok(resp),
                Ok(Err(e)) => {
                    last_err = Some(e);
                    // Only retry on transient errors
                    if matches!(&last_err, Some(PaymentError::InvalidAmount) | Some(PaymentError::MissingApiKey)) {
                        break;
                    }
                }
                Err(_) => {
                    tracing::warn!("⏱️ Timeout create_checkout (attempt {})", attempt);
                    last_err = Some(PaymentError::Stripe("Request timed out".into()));
                }
            }
        }
        Err(last_err.unwrap_or_else(|| PaymentError::Stripe("All retries failed".into())))
    }

    async fn refund_payment(&self, payment_provider_id: &str) -> Result<(), PaymentError> {
        let mut last_err = None;
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = self.base_delay_ms * (1u64 << (attempt - 1));
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            match timeout(Duration::from_millis(self.request_timeout_ms), self.inner.refund_payment(payment_provider_id)).await {
                Ok(Ok(_)) => return Ok(()),
                Ok(Err(e)) => last_err = Some(e),
                Err(_) => last_err = Some(PaymentError::Stripe("Refund timed out".into())),
            }
        }
        Err(last_err.unwrap_or_else(|| PaymentError::Stripe("Refund all retries failed".into())))
    }
}
