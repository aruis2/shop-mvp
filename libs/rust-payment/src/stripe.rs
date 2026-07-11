use async_trait::async_trait;

use crate::{CheckoutResponse, CreateCheckoutRequest, PaymentError, PaymentRepo};

/// Implementare Stripe a `PaymentRepo` — comunică direct cu Stripe API via HTTP
pub struct StripePayment {
    secret_key: String,
}

impl StripePayment {
    pub fn new(secret_key: &str) -> Self {
        Self {
            secret_key: secret_key.to_string(),
        }
    }
}

#[async_trait]
impl PaymentRepo for StripePayment {
    async fn create_checkout(&self, req: CreateCheckoutRequest) -> Result<CheckoutResponse, PaymentError> {
        if req.amount_bani <= 0 {
            return Err(PaymentError::InvalidAmount);
        }
        if self.secret_key.is_empty() {
            return Err(PaymentError::MissingApiKey);
        }

        let client = reqwest::Client::new();

        let product_name = format!("Comanda #{}", &req.order_id[..8.min(req.order_id.len())]);
        let unit_amount = req.amount_bani.to_string();

        // Stripe API folosește form-urlencoded
        let mut form = std::collections::HashMap::new();
        form.insert("mode", "payment");
        form.insert("success_url", &req.success_url);
        form.insert("cancel_url", &req.cancel_url);
        form.insert("line_items[0][price_data][currency]", &req.currency);
        form.insert("line_items[0][price_data][product_data][name]", &product_name);
        form.insert("line_items[0][price_data][unit_amount]", &unit_amount);
        form.insert("line_items[0][quantity]", "1");
        form.insert("metadata[order_id]", &req.order_id);

        let resp = client
            .post("https://api.stripe.com/v1/checkout/sessions")
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .form(&form)
            .send()
            .await
            .map_err(|e| PaymentError::Stripe(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(PaymentError::Stripe(format!("Stripe API error: {}", text)));
        }

        let data: serde_json::Value = resp.json().await
            .map_err(|e| PaymentError::Stripe(e.to_string()))?;

        let checkout_url = data["url"].as_str()
            .ok_or_else(|| PaymentError::Stripe("Missing checkout URL".into()))?
            .to_string();

        let session_id = data["id"].as_str()
            .ok_or_else(|| PaymentError::Stripe("Missing session ID".into()))?
            .to_string();

        Ok(CheckoutResponse {
            checkout_url,
            session_id,
        })
    }

    async fn refund_payment(&self, session_id: &str) -> Result<(), PaymentError> {
        if self.secret_key.is_empty() {
            return Err(PaymentError::MissingApiKey);
        }

        let client = reqwest::Client::new();

        // 1. Recuperează Payment Intent din sesiunea Stripe
        let resp = client
            .get(format!("https://api.stripe.com/v1/checkout/sessions/{}", session_id))
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .send()
            .await
            .map_err(|e| PaymentError::Stripe(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(PaymentError::Stripe(format!("Failed to retrieve session: {}", text)));
        }

        let data: serde_json::Value = resp.json().await
            .map_err(|e| PaymentError::Stripe(e.to_string()))?;

        let payment_intent = data["payment_intent"].as_str()
            .ok_or_else(|| PaymentError::Stripe("No payment_intent in session (not paid yet?)".into()))?
            .to_string();

        // 2. Efectuează refund
        let mut form = std::collections::HashMap::new();
        form.insert("payment_intent", &payment_intent);

        let resp = client
            .post("https://api.stripe.com/v1/refunds")
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .form(&form)
            .send()
            .await
            .map_err(|e| PaymentError::RefundFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(PaymentError::RefundFailed(format!("Stripe refund error: {}", text)));
        }

        Ok(())
    }
}
