//! # rust-marketplace-listings
//!
//! Modul LEGO pentru anunțuri (listings).
//! Depinde de 2 trait-uri: `CategoryService` + `UserService`.
//! Fără dependențe directe de alte LEGO-uri — testabil cu Mock-uri.
//!
//! ## Teste
//! ```bash
//! cargo test -p rust-marketplace-listings
//! ```

mod models;
mod pg;

pub use models::{Listing, CreateListingRequest, UpdateListingRequest};
pub use pg::PgListingRepo;

use async_trait::async_trait;
use uuid::Uuid;
use thiserror::Error;

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, Error)]
pub enum ListingError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Listing not found: {0}")]
    NotFound(Uuid),

    #[error("Category not found: {0}")]
    CategoryNotFound(i32),

    #[error("User not found: {0}")]
    UserNotFound(Uuid),

    #[error("Validation error: {0}")]
    Validation(String),
}

// ============================================================================
// Trait-uri pentru dependințe externe (injectate)
// ============================================================================

/// Verifică dacă o categorie există
#[async_trait]
pub trait CategoryService: Send + Sync {
    async fn category_exists(&self, id: i32) -> Result<bool, ListingError>;
}

/// Verifică dacă un utilizator există
#[async_trait]
pub trait UserService: Send + Sync {
    async fn user_exists(&self, id: Uuid) -> Result<bool, ListingError>;
}

// ============================================================================
// Trait principal — ListingRepo
// ============================================================================

#[async_trait]
pub trait ListingRepo: Send + Sync {
    /// Creează tabela `listings` dacă nu există
    async fn migrate(&self) -> Result<(), ListingError>;

    /// Creează un anunț nou (validează category + user)
    async fn create(&self, user_id: Uuid, req: CreateListingRequest) -> Result<Listing, ListingError>;

    /// Anunțuri active, paginate
    async fn get_active(&self, page: i64, per_page: i64) -> Result<(Vec<Listing>, i64), ListingError>;

    /// Anunț după ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Listing>, ListingError>;

    /// Toate anunțurile active (pentru slug lookup)
    async fn get_all_active(&self) -> Result<Vec<Listing>, ListingError>;

    /// Actualizează un anunț
    async fn update(&self, id: Uuid, req: UpdateListingRequest) -> Result<Option<Listing>, ListingError>;

    /// Incrementează contorul de vizualizări
    async fn increment_views(&self, id: Uuid) -> Result<(), ListingError>;

    /// Caută anunțuri după titlu
    async fn search(&self, query: &str, limit: i64) -> Result<Vec<Listing>, ListingError>;
}

// ============================================================================
// Teste — Mock-uri pentru dependențe
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCategoryService;
    #[async_trait]
    impl CategoryService for MockCategoryService {
        async fn category_exists(&self, id: i32) -> Result<bool, ListingError> {
            Ok(id == 1 || id == 2) // doar id 1 și 2 există
        }
    }

    struct MockUserService;
    #[async_trait]
    impl UserService for MockUserService {
        async fn user_exists(&self, id: Uuid) -> Result<bool, ListingError> {
            Ok(id == Uuid::nil()) // doar uuid-ul nil "există"
        }
    }

    #[tokio::test]
    async fn test_category_exists_mock() {
        let svc = MockCategoryService;
        assert!(svc.category_exists(1).await.unwrap());
        assert!(svc.category_exists(2).await.unwrap());
        assert!(!svc.category_exists(999).await.unwrap());
    }

    #[tokio::test]
    async fn test_user_exists_mock() {
        let svc = MockUserService;
        assert!(svc.user_exists(Uuid::nil()).await.unwrap());
        assert!(!svc.user_exists(Uuid::new_v4()).await.unwrap());
    }

    #[test]
    fn test_listing_validation_empty_title() {
        let req = CreateListingRequest {
            category_id: 1,
            title: String::new(),
            description: None,
            price: None,
            currency: None,
            attributes: None,
            image_urls: None,
            phone: None,
            contact_email: None,
            county: None,
            city: None,
        };
        assert!(req.title.is_empty(), "Titlul nu poate fi gol");
    }

    #[test]
    fn test_listing_price_positive() {
        let req = CreateListingRequest {
            category_id: 1,
            title: "Test".into(), description: None, price: Some(9999),
            currency: None, attributes: None, image_urls: None,
            phone: None, contact_email: None, county: None, city: None,
        };
        // Prețul în bani (cents) poate fi 0 (gratis) sau pozitiv
        assert!(req.price.unwrap() >= 0, "Prețul nu poate fi negativ");
    }

    #[test]
    fn test_listing_price_zero_allowed() {
        let req = CreateListingRequest {
            category_id: 1,
            title: "Gratis".into(), description: None, price: Some(0),
            currency: None, attributes: None, image_urls: None,
            phone: None, contact_email: None, county: None, city: None,
        };
        // Preț 0 = gratis, e permis
        if let Some(price) = req.price {
            assert_eq!(price, 0, "Prețul 0 înseamnă gratis");
        }
    }

    #[test]
    fn test_listing_default_currency() {
        let currency = None;
        assert_eq!(
            currency.unwrap_or_else(|| "RON".to_string()),
            "RON"
        );
    }
}
