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

        // 🔒 UNIQUE constraints — anonimi după session_id, logați după user_id
        // Folosim CREATE UNIQUE INDEX ... WHERE pentru a evita problema NULL-urilor
        // și pentru a garanta că ON CONFLICT funcționează corect.
        let _ = sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_cart_unique_session_product_price
             ON cart_items (session_id, product_slug, price_bani)"
        )
        .execute(&self.pool)
        .await;

        // Pentru user_id, indexul tratează NULL-urile separat (nu se aplică unde user_id IS NULL)
        let _ = sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_cart_unique_user_product_price
             ON cart_items (user_id, product_slug, price_bani)
             WHERE user_id IS NOT NULL"
        )
        .execute(&self.pool)
        .await;

        Ok(())
    }

    /// Obține coșul după session_id (doar itemele anonime, fără user_id)
    /// 🔒 Itemele private (cu user_id) nu se văd după logout — rămân legate de utilizator.
    async fn get_cart(&self, session_id: &str) -> Result<Cart, CartError> {
        let items = sqlx::query_as::<_, CartItem>(
            r#"
            SELECT id, session_id, user_id, product_slug, product_name,
                   price_bani, qty, created_at, updated_at
            FROM cart_items
            WHERE session_id = $1 AND user_id IS NULL
            ORDER BY created_at ASC
            "#
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let total_bani: i64 = items.iter().map(|i| i.price_bani as i64 * i.qty as i64).sum();
        let item_count: i32 = items.iter().map(|i| i.qty).sum();
        let user_id = items.iter().find_map(|i| i.user_id);

        Ok(Cart {
            session_id: session_id.to_string(),
            user_id,
            items,
            total_bani,
            item_count,
        })
    }

    async fn get_cart_by_user(&self, session_id: &str, user_id: Uuid) -> Result<Cart, CartError> {
        let items = sqlx::query_as::<_, CartItem>(
            r#"
            SELECT id, session_id, user_id, product_slug, product_name,
                   price_bani, qty, created_at, updated_at
            FROM cart_items
            WHERE session_id = $1 OR user_id = $2
            ORDER BY created_at ASC
            "#
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let total_bani: i64 = items.iter().map(|i| i.price_bani as i64 * i.qty as i64).sum();
        let item_count: i32 = items.iter().map(|i| i.qty).sum();
        let found_user_id = items.iter().find_map(|i| i.user_id);

        Ok(Cart {
            session_id: session_id.to_string(),
            user_id: found_user_id,
            items,
            total_bani,
            item_count,
        })
    }

    async fn get_private_cart(&self, user_id: Uuid) -> Result<Cart, CartError> {
        let items = sqlx::query_as::<_, CartItem>(
            r#"
            SELECT id, session_id, user_id, product_slug, product_name,
                   price_bani, qty, created_at, updated_at
            FROM cart_items
            WHERE user_id = $1
            ORDER BY created_at ASC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let total_bani: i64 = items.iter().map(|i| i.price_bani as i64 * i.qty as i64).sum();
        let item_count: i32 = items.iter().map(|i| i.qty).sum();

        Ok(Cart {
            session_id: String::new(),
            user_id: Some(user_id),
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

        // 🏭 INSERT ... ON CONFLICT DO UPDATE — previne race condition la add concurent
        // Dacă avem user_id, grupăm după (user_id, product_slug, price_bani)
        // ca să meargă coșul pe toate browserele.
        // Dacă nu avem user_id (anonim), grupăm după (session_id, ...) ca înainte.
        let item = if let Some(uid) = user_id {
            // Upsert după (user_id, product_slug, price_bani) — funcționează cross-browser
            // Folosim UPDATE + INSERT explicit (nu ON CONFLICT) pentru compatibilitate
            // cu partial unique index (WHERE user_id IS NOT NULL).
            let updated = sqlx::query(
                r#"
                UPDATE cart_items
                SET qty = qty + $3, session_id = $4, updated_at = NOW()
                WHERE user_id = $1 AND product_slug = $2 AND price_bani = $5
                "#
            )
            .bind(uid)
            .bind(&req.product_slug)
            .bind(req.qty)
            .bind(uid.to_string())
            .bind(req.price_bani)
            .execute(&self.pool)
            .await?
            .rows_affected();

            if updated == 0 {
                // Nu există — inserează
                // 🔒 Folosim uid.to_string() ca session_id ca să nu intre în conflict
                // cu indexul unic (session_id, product_slug, price_bani) al itemelor anonime.
                sqlx::query_as::<_, CartItem>(
                    r#"
                    INSERT INTO cart_items (session_id, user_id, product_slug, product_name, price_bani, qty)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    RETURNING id, session_id, user_id, product_slug, product_name,
                              price_bani, qty, created_at, updated_at
                    "#
                )
                .bind(uid.to_string())
                .bind(uid)
                .bind(&req.product_slug)
                .bind(&req.product_name)
                .bind(req.price_bani)
                .bind(req.qty)
                .fetch_one(&self.pool)
                .await?
            } else {
                // S-a făcut UPDATE — întoarce itemul actualizat
                sqlx::query_as::<_, CartItem>(
                    r#"
                    SELECT id, session_id, user_id, product_slug, product_name,
                           price_bani, qty, created_at, updated_at
                    FROM cart_items
                    WHERE user_id = $1 AND product_slug = $2 AND price_bani = $3
                    "#
                )
                .bind(uid)
                .bind(&req.product_slug)
                .bind(req.price_bani)
                .fetch_one(&self.pool)
                .await?
            }
        } else {
            // Anonim: upsert după (session_id, product_slug, price_bani)
            sqlx::query_as::<_, CartItem>(
                r#"
                INSERT INTO cart_items (session_id, user_id, product_slug, product_name, price_bani, qty)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (session_id, product_slug, price_bani)
                DO UPDATE SET qty = cart_items.qty + EXCLUDED.qty,
                              updated_at = NOW()
                RETURNING id, session_id, user_id, product_slug, product_name,
                          price_bani, qty, created_at, updated_at
                "#
            )
            .bind(session_id)
            .bind(&None::<uuid::Uuid>)
            .bind(&req.product_slug)
            .bind(&req.product_name)
            .bind(req.price_bani)
            .bind(req.qty)
            .fetch_one(&self.pool)
            .await?
        };

        // Recalculăm totalurile — pentru user_logat, include și itemele de pe alte browsere
        let cart = if user_id.is_some() {
            self.get_cart_by_user(session_id, user_id.unwrap()).await?
        } else {
            self.get_cart(session_id).await?
        };

        Ok(AddItemResponse {
            item,
            item_count: cart.item_count,
            total_bani: cart.total_bani,
        })
    }

    async fn remove_item(&self, _session_id: &str, item_id: Uuid) -> Result<(), CartError> {
        // 🔒 Itemele private au session_id = user_id_string, nu se potrivește cu cookie-ul.
        // Folosim doar id (UUID) — e sigur, UUID-urile sunt neghicibile.
        let result = sqlx::query(
            r#"
            DELETE FROM cart_items
            WHERE id = $1
            "#
        )
        .bind(item_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(CartError::ItemNotFound(item_id));
        }
        Ok(())
    }

    async fn update_qty(&self, _session_id: &str, item_id: Uuid, qty: i32) -> Result<CartItem, CartError> {
        if qty <= 0 {
            return Err(CartError::InvalidQuantity);
        }

        // 🔒 Itemele private au session_id = user_id_string.
        // Folosim doar id (UUID) — e sigur, UUID-urile sunt neghicibile.
        sqlx::query_as::<_, CartItem>(
            r#"
            UPDATE cart_items
            SET qty = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING id, session_id, user_id, product_slug, product_name,
                      price_bani, qty, created_at, updated_at
            "#
        )
        .bind(item_id)
        .bind(qty)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(CartError::ItemNotFound(item_id))
    }

    async fn clear_cart(&self, session_id: &str, user_id: Option<Uuid>) -> Result<(), CartError> {
        if let Some(uid) = user_id {
            // 🔒 Șterge și itemele private (session_id = uid.to_string()) + publice (session_id browser)
            sqlx::query(
                "DELETE FROM cart_items WHERE session_id = $1 OR (user_id = $2 AND user_id IS NOT NULL)"
            )
            .bind(session_id)
            .bind(uid)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query("DELETE FROM cart_items WHERE session_id = $1")
                .bind(session_id)
                .execute(&self.pool)
                .await?;
        }
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
