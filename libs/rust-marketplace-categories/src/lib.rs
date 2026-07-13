//! # rust-marketplace-categories
//!
//! Modul LEGO pentru categorii ierarhice (arbore).
//! Independent, testabil, gata de asamblat în orice proiect Axum.
//!
//! ## LEGO Assembly
//! Cu feature-ul `products`, `PgCategoryRepo` implementează
//! `CategoryService` din `rust-marketplace-products`:
//!
//! ```toml
//! rust-marketplace-categories = { features = ["products"] }
//! ```
//!
//! ```ignore
//! // PgCategoryRepo implementează CategoryService din rust-marketplace-products
//! // când feature-ul "products" e activ:
//! // let cats = PgCategoryRepo::new(pool);
//! // let prods = PgProductRepo::new(pool, Box::new(cats)); // LEGO! 🧱
//! ```
//!
//! ## Teste
//! ```bash
//! cargo test -p rust-marketplace-categories
//! ```

mod models;
mod pg;

pub use models::{Category, CategoryBreadcrumb, CategoryView};
pub use pg::PgCategoryRepo;

use async_trait::async_trait;
use thiserror::Error;

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, Error)]
pub enum CategoryError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Category not found: {0}")]
    NotFound(String),

    #[error("Invalid parent category: {0}")]
    InvalidParent(i32),
}

// ============================================================================
// Trait — singura legătură cu exteriorul
// ============================================================================

#[async_trait]
pub trait CategoryRepo: Send + Sync {
    /// Creează tabela `categories` dacă nu există
    async fn migrate(&self) -> Result<(), CategoryError>;

    /// Obține subcategoriile unui părinte
    async fn get_subcategories(&self, parent_id: i32) -> Result<Vec<Category>, CategoryError>;

    /// Obține o categorie după slug
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Category>, CategoryError>;

    /// Obține o categorie după ID
    async fn get_by_id(&self, id: i32) -> Result<Option<Category>, CategoryError>;

    /// Obține breadcrumb-ul (de la frunză la rădăcină)
    async fn get_breadcrumb(&self, category_id: i32) -> Result<Vec<CategoryBreadcrumb>, CategoryError>;

    /// Obține TOATE categoriile (pentru construirea arborelui)
    async fn get_all(&self) -> Result<Vec<Category>, CategoryError>;

    /// Construiește arborele complet de categorii
    async fn get_tree(&self) -> Result<Vec<CategoryView>, CategoryError> {
        let all = self.get_all().await?;
        Ok(build_tree(all))
    }
}

// ============================================================================
// Funcție ajutătoare — construiește arborele din lista plată
// ============================================================================

pub fn build_tree(all: Vec<Category>) -> Vec<CategoryView> {
    let roots: Vec<CategoryView> = all
        .iter()
        .filter(|c| c.parent_id.is_none())
        .map(|c| CategoryView {
            id: c.id,
            name: c.name.clone(),
            slug: c.slug.clone(),
            icon: c.icon.clone().unwrap_or_default(),
            description: c.description.clone().unwrap_or_default(),
            children: get_children(c.id, &all),
        })
        .collect();
    roots
}

fn get_children(parent_id: i32, all: &[Category]) -> Vec<CategoryView> {
    all.iter()
        .filter(|c| c.parent_id == Some(parent_id))
        .map(|c| CategoryView {
            id: c.id,
            name: c.name.clone(),
            slug: c.slug.clone(),
            icon: c.icon.clone().unwrap_or_default(),
            description: c.description.clone().unwrap_or_default(),
            children: get_children(c.id, all),
        })
        .collect()
}

// ============================================================================
// Teste — fără PostgreSQL, doar logica pură
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_categories() -> Vec<Category> {
        vec![
            Category {
                id: 1, name: "Electronice".into(), slug: "electronice".into(),
                parent_id: None, icon: Some("💻".into()), description: Some(String::new()),
                created_at: None,
            },
            Category {
                id: 2, name: "Laptopuri".into(), slug: "laptopuri".into(),
                parent_id: Some(1), icon: Some("💻".into()), description: Some(String::new()),
                created_at: None,
            },
            Category {
                id: 3, name: "Desktop-uri".into(), slug: "desktop-uri".into(),
                parent_id: Some(1), icon: Some("🖥".into()), description: Some(String::new()),
                created_at: None,
            },
            Category {
                id: 4, name: "Gaming".into(), slug: "gaming".into(),
                parent_id: Some(2), icon: Some("🎮".into()), description: Some(String::new()),
                created_at: None,
            },
        ]
    }

    #[test]
    fn test_build_tree_root_count() {
        let tree = build_tree(mock_categories());
        assert_eq!(tree.len(), 1, "Ar trebui să fie 1 rădăcină: Electronice");
        assert_eq!(tree[0].name, "Electronice");
    }

    #[test]
    fn test_build_tree_children_count() {
        let tree = build_tree(mock_categories());
        assert_eq!(tree[0].children.len(), 2, "Electronice ar trebui să aibă 2 copii");
    }

    #[test]
    fn test_build_tree_nested_depth() {
        let tree = build_tree(mock_categories());
        let laptopuri = &tree[0].children[0];
        assert_eq!(laptopuri.name, "Laptopuri");
        assert_eq!(laptopuri.children.len(), 1, "Laptopuri ar trebui să aibă 1 copil");
        assert_eq!(laptopuri.children[0].name, "Gaming");
    }

    #[test]
    fn test_build_tree_empty() {
        let tree = build_tree(vec![]);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_build_tree_no_root() {
        let cats = vec![
            Category {
                id: 2, name: "Laptopuri".into(), slug: "laptopuri".into(),
                parent_id: Some(1), icon: None, description: None, created_at: None,
            },
        ];
        let tree = build_tree(cats);
        assert!(tree.is_empty(), "Fără rădăcină, arborele ar trebui să fie gol");
    }
}
