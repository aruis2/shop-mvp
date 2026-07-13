//! # rust-marketplace-orders
//!
//! Modul LEGO pentru comenzi și checkout.
//! Gestionează plasarea comenzilor, istoric, statusuri.
//!
//! ## Teste
//! ```bash
//! cargo test -p rust-marketplace-orders
//! ```

mod models;
mod pg;

pub use models::{Order, OrderItem, PlaceOrderRequest};
pub use pg::PgOrderRepo;

use async_trait::async_trait;
use uuid::Uuid;
use thiserror::Error;

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, Error)]
pub enum OrderError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Order not found: {0}")]
    NotFound(Uuid),

    #[error("Empty cart")]
    EmptyCart,

    #[error("Stoc insuficient pentru {0}: {1} disponibil, {2} cerut")]
    InsufficientStock(String, i32, i32),

    #[error("Validation error: {0}")]
    Validation(String),
}

// ============================================================================
// Trait principal — OrderRepo
// ============================================================================

#[async_trait]
pub trait OrderRepo: Send + Sync {
    /// Creează tabelele `orders` + `order_items`
    async fn migrate(&self) -> Result<(), OrderError>;

    /// Plasează o comandă (preia items din cart și le salvează)
    async fn place_order(&self, user_id: Option<Uuid>, req: PlaceOrderRequest, cart_items: Vec<(String, String, i64, i32)>) -> Result<Order, OrderError>;

    /// Comenzile unui utilizator sau sesiuni
    async fn get_orders(&self, session_id: &str) -> Result<Vec<Order>, OrderError>;

    /// Comenzile unui utilizator autentificat (cu paginare)
    async fn get_orders_by_user(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<(Vec<Order>, i64), OrderError>;

    /// O comandă după ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Order>, OrderError>;

    /// Itemii unei comenzi
    async fn get_items(&self, order_id: Uuid) -> Result<Vec<OrderItem>, OrderError>;

    /// Actualizează statusul unei comenzi
    async fn update_status(&self, id: Uuid, status: &str) -> Result<(), OrderError>;

    /// Salvează datele de plată pe o comandă (provider + id-ul lui)
    async fn set_payment_info(&self, id: Uuid, provider: &str, provider_id: &str) -> Result<(), OrderError>;

    /// Actualizează payment_status
    async fn update_payment_status(&self, id: Uuid, payment_status: &str) -> Result<(), OrderError>;

    /// Toate comenzile (pentru admin)
    async fn get_all_orders(&self, limit: i64, offset: i64) -> Result<(Vec<Order>, i64), OrderError>;

    /// 🔒 Idempotency: creează tabela
    async fn migrate_idempotency(&self) -> Result<(), OrderError>;

    /// 🔒 Idempotency: verifică dacă o cheie există deja (returnează rezultatul)
    async fn check_idempotency(&self, key: &str) -> Result<Option<String>, OrderError>;

    /// 🔒 Idempotency: stochează rezultatul pentru o cheie (INSERT ON CONFLICT DO NOTHING)
    async fn store_idempotency(&self, key: &str, result: &str) -> Result<(), OrderError>;

    /// 🔐 Migrează comenzile anonime (user_id IS NULL) la un utilizator autentificat.
    /// Folosit de admin_migrate_orders pentru a asocia comenzi anterioare.
    async fn migrate_user_orders(&self, user_id: Uuid) -> Result<u64, OrderError>;
}

// ============================================================================
// Teste
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_status_constants() {
        assert_eq!(Order::STATUS_PENDING, "pending");
        assert_eq!(Order::STATUS_CONFIRMED, "confirmed");
        assert_eq!(Order::STATUS_SHIPPED, "shipped");
        assert_eq!(Order::STATUS_DELIVERED, "delivered");
        assert_eq!(Order::STATUS_CANCELLED, "cancelled");
    }

    #[test]
    fn test_place_order_request() {
        let req = PlaceOrderRequest {
            session_id: "test-session".into(),
            guest_email: None,
            shipping_name: "John Doe".into(),
            shipping_address: "Str. Mare, Nr. 1".into(),
            shipping_phone: "+37360000000".into(),
            notes: None,
        };
        assert_eq!(req.shipping_name, "John Doe");
        assert!(req.notes.is_none());
    }

    #[test]
    fn test_order_error_messages() {
        let err = OrderError::EmptyCart;
        assert_eq!(format!("{}", err), "Empty cart");

        let err = OrderError::Validation("Name required".into());
        assert!(format!("{}", err).contains("Name required"));
    }
}
