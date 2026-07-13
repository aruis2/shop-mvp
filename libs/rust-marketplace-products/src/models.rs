use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub slug: String,
}

/// Un produs din catalog
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: i32,
    pub brand: String,
    pub name: String,
    pub slug: String,
    pub category_id: i32,
    pub release_year: Option<i32>,
    pub specs: serde_json::Value,
    pub price_new: Option<i32>,
    pub affiliate_url: Option<String>,
    pub image_url: Option<String>,
    pub stock_count: i32,
    pub created_at: Option<DateTime<Utc>>,
}

/// Statistici pentru catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductStats {
    pub total: i64,
    pub brands: Vec<BrandCount>,
    pub full_specs: i64,
    pub with_images: i64,
}

/// Număr de produse per brand
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BrandCount {
    pub brand: String,
    pub cnt: Option<i64>,
}

/// Request pentru creare produs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub brand: String,
    pub name: String,
    pub slug: String,
    pub category_id: i32,
    pub release_year: Option<i32>,
    pub specs: Option<serde_json::Value>,
    pub price_new: Option<i32>,
    pub affiliate_url: Option<String>,
    pub image_url: Option<String>,
    pub stock_count: Option<i32>,
}

/// Request pentru actualizare produs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductRequest {
    pub brand: Option<String>,
    pub name: Option<String>,
    pub stock_count: Option<i32>,
    pub slug: Option<String>,
    pub category_id: Option<i32>,
    pub release_year: Option<i32>,
    pub specs: Option<serde_json::Value>,
    pub price_new: Option<i32>,
    pub affiliate_url: Option<String>,
    pub image_url: Option<String>,
}
