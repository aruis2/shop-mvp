use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Un item din coșul de cumpărături
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CartItem {
    pub id: Uuid,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub product_slug: String,
    pub product_name: String,
    pub price_bani: i64,
    pub qty: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Coșul complet al unui utilizator/sesiune
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cart {
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub items: Vec<CartItem>,
    pub total_bani: i64,
    pub item_count: i32,
}

/// Request pentru adăugare în coș
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCartItemRequest {
    pub product_slug: String,
    pub product_name: String,
    pub price_bani: i64,
    pub qty: i32,
}

/// Request pentru actualizare cantitate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateQtyRequest {
    pub qty: i32,
}

/// Răspuns după adăugare
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddItemResponse {
    pub item: CartItem,
    pub item_count: i32,
    pub total_bani: i64,
}
