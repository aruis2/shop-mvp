use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;
use crate::{CartError, CartRepo};

/// Implementare PostgreSQL a `CartRepo`
pub struct PgCartRepo {
    pool: PgPool,
}

impl PgCartRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CartRepo for PgCartRepo {
    async fn migrate(&self) -> Result<(), CartError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cart_items (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                session_id TEXT NOT NULL,
                user_id UUID,
                product_slug TEXT NOT NULL,
                product_name TEXT NOT NULL,
                price_bani BIGINT NOT NULL,
                qty INTEGER NOT NULL DEFAULT 1,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        let _ = sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_cart_items_session ON cart_items(session_id)"
        )
        .execute(&self.pool)
        .await;

        let _ = sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_cart_items_user ON cart_items(user_id)"
        )
        .execute(&self.pool)
        .await;

        Ok(())
    }

    async fn get_cart(&self, session_id: &str) -> Result<Cart, CartError> {
        let items = sqlx::query_as::<_, CartItem>(
            r#"
            SELECT id, session_id, user_id, product_slug, product_name,
                   price_bani, qty, created_at, updated_at
            FROM cart_items
            WHERE session_id = $1
            ORDER BY created_at ASC
            "#
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let total_bani: i64 = items.iter().map(|i| i.price_bani as i64 * i.qty as i64).sum();
        let item_count: i32 = items.iter().map(|i| i.qty).sum();

        // Determinăm user_id (primul item non-null)
        let user_id = items.iter().find_map(|i| i.user_id);

        Ok(Cart {
            session_id: session_id.to_string(),
            user_id,
            items,
            total_bani,
            item_count,
        })
    }

    async fn add_item(
        &self,
        session_id: &str,
        user_id: Option<Uuid>,
        req: AddCartItemRequest,
    ) -> Result<AddItemResponse, CartError> {
        if req.qty <= 0 {
            return Err(CartError::InvalidQuantity);
        }
        if req.price_bani <= 0 {
            return Err(CartError::InvalidPrice);
        }

        // Încercăm mai întâi să incrementăm cantitatea dacă există deja un rând
        // cu ACELAȘI produs ȘI ACELAȘI preț (prețurile diferite merg pe rânduri separate)
        let updated = sqlx::query_as::<_, CartItem>(
            r#"
            UPDATE cart_items
            SET qty = qty + $3, updated_at = NOW()
            WHERE session_id = $1 AND product_slug = $2 AND price_bani = $4
            RETURNING id, session_id, user_id, product_slug, product_name,
                      price_bani, qty, created_at, updated_at
            "#
        )
        .bind(session_id)
        .bind(&req.product_slug)
        .bind(req.qty)
        .bind(req.price_bani)
        .fetch_optional(&self.pool)
        .await?;

        let item = if let Some(existing) = updated {
            existing
        } else {
            // Altfel, inserăm un item nou (preț diferit față de rândurile existente)
            sqlx::query_as::<_, CartItem>(
                r#"
                INSERT INTO cart_items (session_id, user_id, product_slug, product_name, price_bani, qty)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id, session_id, user_id, product_slug, product_name,
                          price_bani, qty, created_at, updated_at
                "#
            )
            .bind(session_id)
            .bind(user_id)
            .bind(&req.product_slug)
            .bind(&req.product_name)
            .bind(req.price_bani)
            .bind(req.qty)
            .fetch_one(&self.pool)
            .await?
        };

        // Recalculăm totalurile
        let cart = self.get_cart(session_id).await?;

        Ok(AddItemResponse {
            item,
            item_count: cart.item_count,
            total_bani: cart.total_bani,
        })
    }

    async fn remove_item(&self, session_id: &str, item_id: Uuid) -> Result<(), CartError> {
        let result = sqlx::query(
            r#"
            DELETE FROM cart_items
            WHERE id = $1 AND session_id = $2
            "#
        )
        .bind(item_id)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(CartError::ItemNotFound(item_id));
        }
        Ok(())
    }

    async fn update_qty(&self, session_id: &str, item_id: Uuid, qty: i32) -> Result<CartItem, CartError> {
        if qty <= 0 {
            return Err(CartError::InvalidQuantity);
        }

        sqlx::query_as::<_, CartItem>(
            r#"
            UPDATE cart_items
            SET qty = $3, updated_at = NOW()
            WHERE id = $1 AND session_id = $2
            RETURNING id, session_id, user_id, product_slug, product_name,
                      price_bani, qty, created_at, updated_at
            "#
        )
        .bind(item_id)
        .bind(session_id)
        .bind(qty)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(CartError::ItemNotFound(item_id))
    }

    async fn clear_cart(&self, session_id: &str) -> Result<(), CartError> {
        sqlx::query("DELETE FROM cart_items WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn assign_to_user(&self, session_id: &str, user_id: Uuid) -> Result<(), CartError> {
        sqlx::query(
            r#"
            UPDATE cart_items
            SET user_id = $2
            WHERE session_id = $1 AND user_id IS NULL
            "#
        )
        .bind(session_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
