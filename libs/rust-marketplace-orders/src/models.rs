use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// O comandă plasată
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub session_id: String,
    pub guest_email: Option<String>,
    pub status: String,
    pub payment_status: String,
    pub total_bani: i64,
    pub shipping_name: String,
    pub shipping_address: String,
    pub shipping_phone: String,
    pub notes: String,
    pub payment_provider: Option<String>,
    pub payment_provider_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Un item dintr-o comandă
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub product_slug: String,
    pub product_name: String,
    pub price_bani: i64,
    pub qty: i32,
    pub created_at: DateTime<Utc>,
}

/// Request pentru plasare comandă
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOrderRequest {
    pub session_id: String,
    pub guest_email: Option<String>,
    pub shipping_name: String,
    pub shipping_address: String,
    pub shipping_phone: String,
    pub notes: Option<String>,
}

/// Statusuri posibile
impl Order {
    pub const STATUS_PENDING: &'static str = "pending";
    pub const PAYMENT_UNPAID: &'static str = "unpaid";
    pub const PAYMENT_PAID: &'static str = "paid";
    pub const PAYMENT_FAILED: &'static str = "failed";
    pub const STATUS_CONFIRMED: &'static str = "confirmed";
    pub const STATUS_SHIPPED: &'static str = "shipped";
    pub const STATUS_DELIVERED: &'static str = "delivered";
    pub const STATUS_CANCELLED: &'static str = "cancelled";
}
