use async_trait::async_trait;
use sqlx::PgPool;

use crate::models::{BrandCount, Category, CreateProductRequest, Product, ProductStats, UpdateProductRequest};
use crate::{CategoryService, ProductError, ProductRepo};

/// Implementare PostgreSQL a trait-ului `ProductRepo`
pub struct PgProductRepo {
    pool: PgPool,
    #[allow(dead_code)]
    categories: Box<dyn CategoryService>,
}

impl PgProductRepo {
    pub fn new(pool: PgPool, categories: Box<dyn CategoryService>) -> Self {
        Self { pool, categories }
    }
}

#[async_trait]
impl ProductRepo for PgProductRepo {
    async fn migrate(&self) -> Result<(), ProductError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS products (
                id SERIAL PRIMARY KEY,
                brand TEXT NOT NULL,
                name TEXT NOT NULL,
                slug TEXT UNIQUE NOT NULL,
                category_id INTEGER NOT NULL,
                release_year INTEGER,
                specs JSONB NOT NULL DEFAULT '{}',
                price_new INTEGER,
                affiliate_url TEXT,
                image_url TEXT,
                created_at TIMESTAMPTZ DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_products_brand ON products(brand);
            CREATE INDEX IF NOT EXISTS idx_products_slug ON products(slug);
            CREATE INDEX IF NOT EXISTS idx_products_category ON products(category_id);
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_products(
        &self,
        brand: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Product>, i64), ProductError> {
        let offset = (page - 1) * per_page;

        let (products, total): (Vec<Product>, i64) = if let Some(b) = brand {
            let products = sqlx::query_as::<_, Product>(
                r#"
                SELECT id, brand, name, slug, category_id, release_year,
                       specs, price_new, affiliate_url, image_url, created_at,
                   stock_count
                FROM products
                WHERE brand = $1
                ORDER BY release_year DESC, name ASC
                LIMIT $2 OFFSET $3
                "#
            )
            .bind(b)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

            let total: i64 =
                sqlx::query_scalar::<_, Option<i64>>("SELECT COUNT(*) FROM products WHERE brand = $1")
                    .bind(b)
                    .fetch_one(&self.pool)
                    .await?
                    .unwrap_or(0);

            (products, total)
        } else {
            let products = sqlx::query_as::<_, Product>(
                r#"
                SELECT id, brand, name, slug, category_id, release_year,
                       specs, price_new, affiliate_url, image_url, created_at,
                   stock_count
                FROM products
                ORDER BY id DESC
                LIMIT $1 OFFSET $2
                "#
            )
            .bind(per_page)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

            let total: i64 =
                sqlx::query_scalar::<_, Option<i64>>("SELECT COUNT(*) FROM products")
                    .fetch_one(&self.pool)
                    .await?
                    .unwrap_or(0);

            (products, total)
        };

        Ok((products, total))
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<Product>, ProductError> {
        let product = sqlx::query_as::<_, Product>(
            r#"
            SELECT id, brand, name, slug, category_id, release_year,
                   specs, price_new, affiliate_url, image_url, created_at,
                   stock_count
            FROM products
            WHERE slug = $1
            "#
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(product)
    }

    async fn get_stats(&self) -> Result<ProductStats, ProductError> {
        let total: i64 =
            sqlx::query_scalar::<_, Option<i64>>("SELECT COUNT(*) FROM products")
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0);

        let brands: Vec<BrandCount> = sqlx::query_as(
            "SELECT brand, COUNT(*) as cnt FROM products GROUP BY brand ORDER BY cnt DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let full_specs: i64 =
            sqlx::query_scalar::<_, Option<i64>>("SELECT COUNT(*) FROM products WHERE specs ? 'cpu'")
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0);

        let with_images: i64 =
            sqlx::query_scalar::<_, Option<i64>>("SELECT COUNT(*) FROM products WHERE image_url IS NOT NULL")
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0);

        Ok(ProductStats {
            total,
            brands,
            full_specs,
            with_images,
        })
    }

    async fn get_brands(&self) -> Result<Vec<String>, ProductError> {
        let brands: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT brand FROM products ORDER BY brand"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(brands)
    }

    async fn get_categories(&self) -> Result<Vec<Category>, ProductError> {
        let categories = sqlx::query_as::<_, Category>(
            "SELECT id, name, slug FROM categories ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(categories)
    }

    async fn search_products(&self, query: &str, page: i64, per_page: i64) -> Result<(Vec<Product>, i64), ProductError> {
        let offset = (page - 1) * per_page;
        let pattern = format!("%{}%", query);

        let products = sqlx::query_as::<_, Product>(
            r#"
            SELECT id, brand, name, slug, category_id, release_year,
                   specs, price_new, affiliate_url, image_url, created_at,
                   stock_count
            FROM products
            WHERE name ILIKE $1 OR brand ILIKE $1 OR slug ILIKE $1
            ORDER BY
                CASE WHEN brand ILIKE $2 THEN 0 ELSE 1 END,
                CASE WHEN name ILIKE $2 THEN 0 ELSE 1 END,
                release_year DESC
            LIMIT $3 OFFSET $4
            "#
        )
        .bind(&pattern)
        .bind(query)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 =
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COUNT(*) FROM products WHERE name ILIKE $1 OR brand ILIKE $1 OR slug ILIKE $1"
            )
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);

        Ok((products, total))
    }

    async fn create_product(&self, req: CreateProductRequest) -> Result<Product, ProductError> {
        if req.brand.is_empty() || req.name.is_empty() {
            return Err(ProductError::Validation("Brand and name are required".into()));
        }

        let product = sqlx::query_as::<_, Product>(
            r#"
            INSERT INTO products (brand, name, slug, category_id, release_year, specs, price_new, affiliate_url, image_url, stock_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, brand, name, slug, category_id, release_year,
                      specs, price_new, affiliate_url, image_url, created_at,
                   stock_count
            "#
        )
        .bind(&req.brand)
        .bind(&req.name)
        .bind(&req.slug)
        .bind(req.category_id)
        .bind(req.release_year)
        .bind(&req.specs.unwrap_or(serde_json::json!({})))
        .bind(req.price_new)
        .bind(&req.affiliate_url)
        .bind(&req.image_url)
        .bind(req.stock_count)
        .fetch_one(&self.pool)
        .await?;

        Ok(product)
    }

    async fn update_product(&self, slug: &str, req: UpdateProductRequest) -> Result<Product, ProductError> {
        // 🔒 Tranzacție cu FOR UPDATE — previne lost update la admin concurent
        let mut tx = self.pool.begin().await?;

        let existing = sqlx::query_as::<_, Product>(
            r#"SELECT id, brand, name, slug, category_id, release_year,
                      specs, price_new, affiliate_url, image_url, created_at,
                   stock_count
            FROM products WHERE slug = $1 FOR UPDATE"#
        )
        .bind(slug)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ProductError::NotFound(slug.to_string()))?;

        let brand = req.brand.unwrap_or(existing.brand);
        let name = req.name.unwrap_or(existing.name);
        let new_slug = req.slug.unwrap_or(existing.slug);
        let category_id = req.category_id.unwrap_or(existing.category_id);
        let release_year = req.release_year.or(existing.release_year);
        let specs = req.specs.unwrap_or(existing.specs);
        let price_new = req.price_new.or(existing.price_new);
        let stock_count = req.stock_count.or(Some(existing.stock_count));
        let affiliate_url = req.affiliate_url.or(existing.affiliate_url);
        let image_url = req.image_url.or(existing.image_url);

        let product = sqlx::query_as::<_, Product>(
            r#"
            UPDATE products
            SET brand = $1, name = $2, slug = $3, category_id = $4,
                release_year = $5, specs = $6, price_new = $7,
                affiliate_url = $8, image_url = $9,
                stock_count = $10
            WHERE slug = $11
            RETURNING id, brand, name, slug, category_id, release_year,
                      specs, price_new, affiliate_url, image_url, created_at,
                   stock_count
            "#
        )
        .bind(&brand)
        .bind(&name)
        .bind(&new_slug)
        .bind(category_id)
        .bind(release_year)
        .bind(&specs)
        .bind(price_new)
        .bind(&affiliate_url)
        .bind(&image_url)
        .bind(stock_count)
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(product)
    }

    async fn delete_product(&self, slug: &str) -> Result<(), ProductError> {
        let result = sqlx::query("DELETE FROM products WHERE slug = $1")
            .bind(slug)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(ProductError::NotFound(slug.to_string()));
        }
        Ok(())
    }
}
