//! # rust-cart
//!
//! Coș de cumpărături persistent (PostgreSQL).
//! Suportă sesiuni anonime (session_id) și utilizatori autentificați (user_id).
//!
//! ## Teste
//! ```bash
//! cargo test -p rust-cart
//! ```

mod models;
mod pg;

pub use models::{CartItem, Cart, AddCartItemRequest, UpdateQtyRequest, AddItemResponse};
pub use pg::PgCartRepo;

use async_trait::async_trait;
use uuid::Uuid;
use thiserror::Error;

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, Error)]
pub enum CartError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Cart item not found: {0}")]
    ItemNotFound(Uuid),

    #[error("Quantity must be positive")]
    InvalidQuantity,

    #[error("Price must be positive")]
    InvalidPrice,
}

// ============================================================================
// Trait principal — CartRepo
// ============================================================================

#[async_trait]
pub trait CartRepo: Send + Sync {
    /// Creează tabela `cart_items` dacă nu există
    async fn migrate(&self) -> Result<(), CartError>;

    /// Obține coșul unei sesiuni (sau utilizator)
    async fn get_cart(&self, session_id: &str) -> Result<Cart, CartError>;

    /// Adaugă un produs în coș (sau incrementează cantitatea dacă există deja)
    async fn add_item(&self, session_id: &str, user_id: Option<Uuid>, req: AddCartItemRequest) -> Result<AddItemResponse, CartError>;

    /// Șterge un item din coș
    async fn remove_item(&self, session_id: &str, item_id: Uuid) -> Result<(), CartError>;

    /// Actualizează cantitatea unui item
    async fn update_qty(&self, session_id: &str, item_id: Uuid, qty: i32) -> Result<CartItem, CartError>;

    /// Golește coșul
    async fn clear_cart(&self, session_id: &str) -> Result<(), CartError>;

    /// Asociază un coș anonim cu un utilizator autentificat (după login)
    async fn assign_to_user(&self, session_id: &str, user_id: Uuid) -> Result<(), CartError>;
}

// ============================================================================
// Teste
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cart_item_defaults() {
        let item = CartItem {
            id: Uuid::nil(),
            session_id: "test-session".into(),
            user_id: None,
            product_slug: "test-product".into(),
            product_name: "Test Product".into(),
            price_bani: 1000,
            qty: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert_eq!(item.qty, 1);
        assert_eq!(item.price_bani, 1000);
        assert_eq!(item.product_name, "Test Product");
    }

    #[test]
    fn test_add_item_response() {
        let item = CartItem {
            id: Uuid::nil(),
            session_id: "s".into(),
            user_id: None,
            product_slug: "p".into(),
            product_name: "P".into(),
            price_bani: 500,
            qty: 2,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let resp = AddItemResponse {
            item: item.clone(),
            item_count: 2,
            total_bani: 1000,
        };
        assert_eq!(resp.item_count, 2);
        assert_eq!(resp.total_bani, 1000);
        assert_eq!(resp.item.product_slug, "p");
    }

    #[test]
    fn test_cart_compute_totals() {
        let item1 = CartItem {
            id: Uuid::nil(),
            session_id: "s".into(),
            user_id: None,
            product_slug: "a".into(),
            product_name: "A".into(),
            price_bani: 1000,
            qty: 2,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let item2 = CartItem {
            id: Uuid::nil(),
            session_id: "s".into(),
            user_id: None,
            product_slug: "b".into(),
            product_name: "B".into(),
            price_bani: 500,
            qty: 3,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let cart = Cart {
            session_id: "s".into(),
            user_id: None,
            items: vec![item1, item2],
            total_bani: 3500,  // 1000*2 + 500*3
            item_count: 5,     // 2 + 3
        };
        assert_eq!(cart.total_bani, 3500);
        assert_eq!(cart.item_count, 5);
    }

    #[test]
    fn test_cart_empty() {
        let cart = Cart {
            session_id: "empty".into(),
            user_id: None,
            items: vec![],
            total_bani: 0,
            item_count: 0,
        };
        assert!(cart.items.is_empty());
        assert_eq!(cart.total_bani, 0);
        assert_eq!(cart.item_count, 0);
    }
}
