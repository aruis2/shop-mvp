//! # rust-marketplace-products
//!
//! Modul LEGO pentru catalog de produse cu filtrare pe brand.
//! Depinde de un trait `CategoryService` — nu știe de implementarea reală.
//!
//! ## Teste
//! ```bash
//! cargo test -p rust-marketplace-products
//! ```

mod models;
mod pg;

pub use models::{BrandCount, Category, CreateProductRequest, UpdateProductRequest, Product, ProductStats};
pub use pg::PgProductRepo;

use async_trait::async_trait;
use thiserror::Error;

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, Error)]
pub enum ProductError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Product not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

// ============================================================================
// Trait pentru categorii (dependență injectată — Level 2 coupling)
// ============================================================================

/// Singura dependență externă de care are nevoie ProductRepo.
/// Poți implementa cu PostgreSQL, mock, sau orice.
#[async_trait]
pub trait CategoryService: Send + Sync {
    async fn category_exists(&self, category_id: i32) -> Result<bool, ProductError>;
}

// ============================================================================
// Trait principal — ProductRepo
// ============================================================================

#[async_trait]
pub trait ProductRepo: Send + Sync {
    /// Creează tabela `products` dacă nu există
    async fn migrate(&self) -> Result<(), ProductError>;

    /// Produse paginate, opțional filtrate pe brand
    async fn get_products(
        &self,
        brand: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Product>, i64), ProductError>;

    /// Produs după slug
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Product>, ProductError>;

    /// Statistici catalog
    async fn get_stats(&self) -> Result<ProductStats, ProductError>;

    /// Toate brandurile disponibile
    async fn get_brands(&self) -> Result<Vec<String>, ProductError>;

    async fn get_categories(&self) -> Result<Vec<Category>, ProductError>;

    /// Caută produse după text (nume, brand, slug)
    async fn search_products(&self, query: &str, page: i64, per_page: i64) -> Result<(Vec<Product>, i64), ProductError>;

    /// Creează un produs nou
    async fn create_product(&self, req: CreateProductRequest) -> Result<Product, ProductError>;

    /// Actualizează un produs existent
    async fn update_product(&self, slug: &str, req: UpdateProductRequest) -> Result<Product, ProductError>;

    /// Șterge un produs
    async fn delete_product(&self, slug: &str) -> Result<(), ProductError>;
}

// ============================================================================
// Teste — fără PostgreSQL, doar logica paginării
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Câte produse pe pagină
    const PER_PAGE: i64 = 24;

    fn total_pages(total: i64) -> i64 {
        if total == 0 { 1 } else { (total as f64 / PER_PAGE as f64).ceil() as i64 }
    }

    fn build_page_numbers(current: i64, total: i64) -> Vec<i64> {
        // Colectăm toate paginile care trebuie afișate
        let mut show: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();

        // Primele 3 pagini
        for p in 1..=3.min(total) { show.insert(p); }

        // Ultimele 3 pagini
        for p in (total - 2).max(1)..=total { show.insert(p); }

        // Fereastra în jurul paginii curente
        for p in (current - 2).max(1)..=(current + 2).min(total) { show.insert(p); }

        // Transformăm în vector cu ellipsis
        let mut pages = Vec::new();
        let mut prev = 0i64;
        for &p in &show {
            if prev != 0 && p > prev + 1 {
                pages.push(-1); // ellipsis
            }
            pages.push(p);
            prev = p;
        }
        pages
    }

    #[test]
    fn test_total_pages_exact() {
        assert_eq!(total_pages(24), 1, "24 produse = 1 pagină");
        assert_eq!(total_pages(48), 2, "48 produse = 2 pagini");
    }

    #[test]
    fn test_total_pages_round_up() {
        assert_eq!(total_pages(25), 2, "25 produse = 2 pagini (rotunjire)");
        assert_eq!(total_pages(1), 1);
    }

    #[test]
    fn test_total_pages_zero() {
        assert_eq!(total_pages(0), 1, "0 produse = 1 pagină goală");
    }

    #[test]
    fn test_page_numbers_first_page() {
        let pages = build_page_numbers(1, 10);
        assert_eq!(pages, vec![1, 2, 3, -1, 8, 9, 10]);
    }

    #[test]
    fn test_page_numbers_middle() {
        let pages = build_page_numbers(5, 10);
        // 7 și 8 sunt consecutive — nu apare ellipsis între ele
        assert_eq!(pages, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_page_numbers_last_page() {
        let pages = build_page_numbers(10, 10);
        assert_eq!(pages, vec![1, 2, 3, -1, 8, 9, 10]);
    }

    #[test]
    fn test_page_numbers_few_pages() {
        let pages = build_page_numbers(1, 3);
        assert_eq!(pages, vec![1, 2, 3], "3 pagini = fără ellipsis");
    }

    #[test]
    fn test_page_numbers_many_first() {
        let pages = build_page_numbers(1, 20);
        assert_eq!(pages, vec![1, 2, 3, -1, 18, 19, 20]);
        // Notă: pentru pagina 1, fereastra (1±2) e în primele 3 — deci fără gap
    }

    #[test]
    fn test_page_numbers_many_middle() {
        let pages = build_page_numbers(10, 20);
        assert_eq!(pages, vec![1, 2, 3, -1, 8, 9, 10, 11, 12, -1, 18, 19, 20]);
    }
}
