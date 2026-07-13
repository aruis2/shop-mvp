use async_trait::async_trait;
use sqlx::PgPool;

use crate::models::{Category, CategoryBreadcrumb};
use crate::{CategoryError, CategoryRepo};

/// Implementare PostgreSQL a trait-ului `CategoryRepo`
pub struct PgCategoryRepo {
    pool: PgPool,
}

impl PgCategoryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CategoryRepo for PgCategoryRepo {
    async fn migrate(&self) -> Result<(), CategoryError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS categories (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                slug TEXT UNIQUE NOT NULL,
                parent_id INTEGER REFERENCES categories(id) ON DELETE CASCADE,
                icon TEXT,
                description TEXT,
                created_at TIMESTAMPTZ DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_categories_parent_id ON categories(parent_id);
            CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_subcategories(&self, parent_id: i32) -> Result<Vec<Category>, CategoryError> {
        let categories = sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, parent_id, icon, description, created_at
            FROM categories
            WHERE parent_id = $1
            ORDER BY name
            "#
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(categories)
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<Category>, CategoryError> {
        let category = sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, parent_id, icon, description, created_at
            FROM categories
            WHERE slug = $1
            "#
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(category)
    }

    async fn get_by_id(&self, id: i32) -> Result<Option<Category>, CategoryError> {
        let category = sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, parent_id, icon, description, created_at
            FROM categories
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(category)
    }

    async fn get_breadcrumb(&self, category_id: i32) -> Result<Vec<CategoryBreadcrumb>, CategoryError> {
        let breadcrumb = sqlx::query_as::<_, CategoryBreadcrumb>(
            r#"
            WITH RECURSIVE category_path AS (
                SELECT c.id, c.name, c.slug, c.parent_id, 1 as level
                FROM categories c
                WHERE c.id = $1
                UNION ALL
                SELECT c.id, c.name, c.slug, c.parent_id, cp.level + 1
                FROM categories c
                JOIN category_path cp ON c.id = cp.parent_id
            )
            SELECT cp.id, cp.name, cp.slug, cp.level
            FROM category_path cp
            ORDER BY cp.level DESC
            "#
        )
        .bind(category_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(breadcrumb)
    }

    async fn get_all(&self) -> Result<Vec<Category>, CategoryError> {
        let categories = sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, parent_id, icon, description, created_at
            FROM categories
            ORDER BY parent_id NULLS FIRST, name
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(categories)
    }
}

// ============================================================================
// LEGO: PgCategoryRepo ca CategoryService (când feature "products" e activ)
// ============================================================================

#[cfg(feature = "products")]
#[async_trait]
impl rust_marketplace_products::CategoryService for PgCategoryRepo {
    async fn category_exists(&self, category_id: i32) -> Result<bool, rust_marketplace_products::ProductError> {
        match self.get_by_id(category_id).await {
            Ok(cat) => Ok(cat.is_some()),
            Err(e) => Err(rust_marketplace_products::ProductError::Database(
                sqlx::Error::Protocol(e.to_string()),
            )),
        }
    }
}
