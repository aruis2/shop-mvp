use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Un anunț din marketplace
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Listing {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub price: Option<i32>,
    pub currency: String,
    pub attributes: serde_json::Value,
    pub image_urls: Vec<String>,
    pub phone: Option<String>,
    pub contact_email: Option<String>,
    pub county: Option<String>,
    pub city: Option<String>,
    pub status: String,
    pub views: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Request pentru creare anunț
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListingRequest {
    pub category_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub price: Option<i32>,
    pub currency: Option<String>,
    pub attributes: Option<serde_json::Value>,
    pub image_urls: Option<Vec<String>>,
    pub phone: Option<String>,
    pub contact_email: Option<String>,
    pub county: Option<String>,
    pub city: Option<String>,
}

/// Request pentru actualizare anunț
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListingRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub price: Option<i32>,
    pub currency: Option<String>,
    pub attributes: Option<serde_json::Value>,
    pub image_urls: Option<Vec<String>>,
    pub phone: Option<String>,
    pub contact_email: Option<String>,
    pub county: Option<String>,
    pub city: Option<String>,
    pub status: Option<String>,
}
