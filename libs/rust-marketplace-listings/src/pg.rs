use async_trait::async_trait;
use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;

use crate::models::{Listing, CreateListingRequest, UpdateListingRequest};
use crate::{CategoryService, ListingError, ListingRepo, UserService};

/// Implementare PostgreSQL a `ListingRepo`
pub struct PgListingRepo {
    pool: PgPool,
    categories: Box<dyn CategoryService>,
    users: Box<dyn UserService>,
}

impl PgListingRepo {
    pub fn new(
        pool: PgPool,
        categories: Box<dyn CategoryService>,
        users: Box<dyn UserService>,
    ) -> Self {
        Self { pool, categories, users }
    }
}

#[async_trait]
impl ListingRepo for PgListingRepo {
    async fn migrate(&self) -> Result<(), ListingError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS listings (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL,
                category_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                price INTEGER,
                currency TEXT DEFAULT 'RON',
                attributes JSONB DEFAULT '{}',
                image_urls TEXT[] DEFAULT '{}',
                phone TEXT,
                contact_email TEXT,
                county TEXT,
                city TEXT,
                status TEXT DEFAULT 'active',
                views INTEGER DEFAULT 0,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW(),
                expires_at TIMESTAMPTZ DEFAULT NOW() + INTERVAL '30 days'
            );
            CREATE INDEX IF NOT EXISTS idx_listings_status ON listings(status);
            CREATE INDEX IF NOT EXISTS idx_listings_category ON listings(category_id);
            CREATE INDEX IF NOT EXISTS idx_listings_user ON listings(user_id);
            CREATE INDEX IF NOT EXISTS idx_listings_created ON listings(created_at DESC);
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create(&self, user_id: Uuid, req: CreateListingRequest) -> Result<Listing, ListingError> {
        // Validare: categoria există?
        if !self.categories.category_exists(req.category_id).await? {
            return Err(ListingError::CategoryNotFound(req.category_id));
        }

        // Validare: userul există?
        if !self.users.user_exists(user_id).await? {
            return Err(ListingError::UserNotFound(user_id));
        }

        // Validare: titlu ne gol
        if req.title.trim().is_empty() {
            return Err(ListingError::Validation("Title cannot be empty".into()));
        }

        let listing = sqlx::query_as::<_, Listing>(
            r#"
            INSERT INTO listings (
                user_id, category_id, title, description, price, currency,
                attributes, image_urls, phone, contact_email, county, city
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, user_id, category_id, title, description, price,
                      currency, attributes, image_urls, phone, contact_email,
                      county, city, status, views, created_at, updated_at, expires_at
            "#
        )
        .bind(user_id)
        .bind(req.category_id)
        .bind(req.title)
        .bind(req.description)
        .bind(req.price)
        .bind(req.currency.unwrap_or_else(|| "RON".to_string()))
        .bind(req.attributes.unwrap_or_else(|| serde_json::json!({})))
        .bind(req.image_urls.unwrap_or_default())
        .bind(req.phone)
        .bind(req.contact_email)
        .bind(req.county)
        .bind(req.city)
        .fetch_one(&self.pool)
        .await?;

        Ok(listing)
    }

    async fn get_active(&self, page: i64, per_page: i64) -> Result<(Vec<Listing>, i64), ListingError> {
        let offset = (page - 1) * per_page;

        let listings = sqlx::query_as::<_, Listing>(
            r#"
            SELECT id, user_id, category_id, title, description, price,
                   currency, attributes, image_urls, phone, contact_email,
                   county, city, status, views, created_at, updated_at, expires_at
            FROM listings
            WHERE status = 'active'
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT COUNT(*) FROM listings WHERE status = 'active'"
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok((listings, total))
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Listing>, ListingError> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"
            SELECT id, user_id, category_id, title, description, price,
                   currency, attributes, image_urls, phone, contact_email,
                   county, city, status, views, created_at, updated_at, expires_at
            FROM listings
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(listing)
    }

    async fn get_all_active(&self) -> Result<Vec<Listing>, ListingError> {
        let listings = sqlx::query_as::<_, Listing>(
            r#"
            SELECT id, user_id, category_id, title, description, price,
                   currency, attributes, image_urls, phone, contact_email,
                   county, city, status, views, created_at, updated_at, expires_at
            FROM listings
            WHERE status = 'active'
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(listings)
    }

    async fn update(&self, id: Uuid, req: UpdateListingRequest) -> Result<Option<Listing>, ListingError> {
        // Folosim QueryBuilder pentru update dinamic
        let mut qb = QueryBuilder::new("UPDATE listings SET ");
        let mut sep = qb.separated(", ");

        if let Some(title) = req.title {
            sep.push("title = ");
            sep.push_bind(title);
        }
        if let Some(description) = req.description {
            sep.push("description = ");
            sep.push_bind(description);
        }
        if let Some(price) = req.price {
            sep.push("price = ");
            sep.push_bind(price);
        }
        if let Some(currency) = req.currency {
            sep.push("currency = ");
            sep.push_bind(currency);
        }
        if let Some(attributes) = req.attributes {
            sep.push("attributes = ");
            sep.push_bind(attributes);
        }
        if let Some(image_urls) = req.image_urls {
            sep.push("image_urls = ");
            sep.push_bind(image_urls);
        }
        if let Some(phone) = req.phone {
            sep.push("phone = ");
            sep.push_bind(phone);
        }
        if let Some(contact_email) = req.contact_email {
            sep.push("contact_email = ");
            sep.push_bind(contact_email);
        }
        if let Some(county) = req.county {
            sep.push("county = ");
            sep.push_bind(county);
        }
        if let Some(city) = req.city {
            sep.push("city = ");
            sep.push_bind(city);
        }
        if let Some(status) = req.status {
            sep.push("status = ");
            sep.push_bind(status);
        }

        sep.push("updated_at = NOW()");
        qb.push(" WHERE id = ");
        qb.push_bind(id);
        qb.push(" RETURNING id, user_id, category_id, title, description, price,
                   currency, attributes, image_urls, phone, contact_email,
                   county, city, status, views, created_at, updated_at, expires_at");

        let listing = qb.build_query_as::<Listing>()
            .fetch_optional(&self.pool)
            .await?;

        Ok(listing)
    }

    async fn increment_views(&self, id: Uuid) -> Result<(), ListingError> {
        sqlx::query("UPDATE listings SET views = views + 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn search(&self, query: &str, limit: i64) -> Result<Vec<Listing>, ListingError> {
        let listings = sqlx::query_as::<_, Listing>(
            r#"
            SELECT id, user_id, category_id, title, description, price,
                   currency, attributes, image_urls, phone, contact_email,
                   county, city, status, views, created_at, updated_at, expires_at
            FROM listings
            WHERE status = 'active' AND title ILIKE $1
            ORDER BY created_at DESC
            LIMIT $2
            "#
        )
        .bind(format!("%{}%", query))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(listings)
    }
}


