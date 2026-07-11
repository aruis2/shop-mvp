use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;
use crate::{OrderError, OrderRepo};

/// Implementare PostgreSQL a `OrderRepo`
pub struct PgOrderRepo {
    pool: PgPool,
}

impl PgOrderRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrderRepo for PgOrderRepo {
    async fn migrate(&self) -> Result<(), OrderError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS orders (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID,
                session_id TEXT NOT NULL,
                guest_email TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                total_bani BIGINT NOT NULL DEFAULT 0,
                shipping_name TEXT NOT NULL,
                shipping_address TEXT NOT NULL,
                shipping_phone TEXT NOT NULL,
                notes TEXT NOT NULL DEFAULT '',
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        let _ = sqlx::query(r#"
            ALTER TABLE orders ADD COLUMN IF NOT EXISTS payment_status TEXT NOT NULL DEFAULT 'unpaid'
        "#).execute(&self.pool).await;

        let _ = sqlx::query(r#"
            ALTER TABLE orders ADD COLUMN IF NOT EXISTS payment_provider TEXT DEFAULT 'stripe'
        "#).execute(&self.pool).await;
        let _ = sqlx::query(r#"
            ALTER TABLE orders ADD COLUMN IF NOT EXISTS payment_provider_id TEXT
        "#).execute(&self.pool).await;

        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_session ON orders(session_id)")
            .execute(&self.pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_user ON orders(user_id)")
            .execute(&self.pool).await;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS order_items (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
                product_slug TEXT NOT NULL,
                product_name TEXT NOT NULL,
                price_bani BIGINT NOT NULL,
                qty INTEGER NOT NULL DEFAULT 1,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id)")
            .execute(&self.pool).await;

        Ok(())
    }

    async fn place_order(
        &self,
        user_id: Option<Uuid>,
        req: PlaceOrderRequest,
        cart_items: Vec<(String, String, i64, i32)>,
    ) -> Result<Order, OrderError> {
        if cart_items.is_empty() {
            return Err(OrderError::EmptyCart);
        }
        if req.shipping_name.trim().is_empty() || req.shipping_address.trim().is_empty() {
            return Err(OrderError::Validation("Name and address are required".into()));
        }

        // 🔒 Tranzacție atomică: verifică stoc → decrementează → creează comandă
        let mut tx = self.pool.begin().await?;

        let total_bani: i64 = cart_items.iter().map(|(_, _, price, qty)| price * *qty as i64).sum();
        let notes = req.notes.unwrap_or_default();

        // Verifică și blochează stocul pentru fiecare produs
        for (slug, _name, _price, qty) in &cart_items {
            let row: Option<(i32,)> = sqlx::query_as(
                r#"SELECT stock_count FROM products WHERE slug = $1 FOR UPDATE"#
            )
            .bind(slug)
            .fetch_optional(&mut *tx)
            .await?;

            match row {
                Some((stock,)) if stock >= *qty => {
                    // Decrementează stocul
                    sqlx::query(
                        r#"UPDATE products SET stock_count = stock_count - $1 WHERE slug = $2"#
                    )
                    .bind(qty)
                    .bind(slug)
                    .execute(&mut *tx)
                    .await?;
                }
                Some((stock,)) => {
                    return Err(OrderError::InsufficientStock(slug.clone(), stock, *qty));
                }
                None => {
                    return Err(OrderError::InsufficientStock(slug.clone(), 0, *qty));
                }
            }
        }

        let order = sqlx::query_as::<_, Order>(
            r#"
            INSERT INTO orders (user_id, session_id, guest_email, status, payment_status, total_bani, shipping_name, shipping_address, shipping_phone, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, user_id, session_id, guest_email, status, payment_status, total_bani,
                      shipping_name, shipping_address, shipping_phone, notes,
                      payment_provider, payment_provider_id, created_at, updated_at
            "#
        )
        .bind(user_id)
        .bind(&req.session_id)
        .bind(&req.guest_email)
        .bind(Order::STATUS_PENDING)
        .bind(Order::PAYMENT_UNPAID)
        .bind(total_bani)
        .bind(&req.shipping_name)
        .bind(&req.shipping_address)
        .bind(&req.shipping_phone)
        .bind(&notes)
        .fetch_one(&mut *tx)
        .await?;

        // Inserează itemii
        for (slug, name, price, qty) in &cart_items {
            sqlx::query(
                r#"
                INSERT INTO order_items (order_id, product_slug, product_name, price_bani, qty)
                VALUES ($1, $2, $3, $4, $5)
                "#
            )
            .bind(order.id)
            .bind(&slug)
            .bind(&name)
            .bind(price)
            .bind(qty)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(order)
    }

    async fn get_orders(&self, session_id: &str) -> Result<Vec<Order>, OrderError> {
        let orders = sqlx::query_as::<_, Order>(
            r#"
            SELECT id, user_id, session_id, guest_email, status, payment_status, total_bani,
                   shipping_name, shipping_address, shipping_phone, notes,
                      payment_provider, payment_provider_id, created_at, updated_at
            FROM orders
            WHERE session_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(orders)
    }

    async fn get_orders_by_user(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<(Vec<Order>, i64), OrderError> {
        let orders = sqlx::query_as::<_, Order>(
            r#"
            SELECT id, user_id, session_id, guest_email, status, payment_status, total_bani,
                   shipping_name, shipping_address, shipping_phone, notes,
                   payment_provider, payment_provider_id, created_at, updated_at
            FROM orders
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT COUNT(*) FROM orders WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok((orders, total))
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Order>, OrderError> {
        let order = sqlx::query_as::<_, Order>(
            r#"
            SELECT id, user_id, session_id, guest_email, status, payment_status, total_bani,
                   shipping_name, shipping_address, shipping_phone, notes,
                   payment_provider, payment_provider_id, created_at, updated_at
            FROM orders WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(order)
    }

    async fn get_items(&self, order_id: Uuid) -> Result<Vec<OrderItem>, OrderError> {
        let items = sqlx::query_as::<_, OrderItem>(
            r#"
            SELECT id, order_id, product_slug, product_name, price_bani, qty, created_at
            FROM order_items WHERE order_id = $1
            ORDER BY created_at ASC
            "#
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(items)
    }

    async fn update_status(&self, id: Uuid, status: &str) -> Result<(), OrderError> {
        let result = sqlx::query(
            "UPDATE orders SET status = $2, updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .bind(status)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(OrderError::NotFound(id));
        }
        Ok(())
    }

    async fn set_payment_info(&self, id: Uuid, provider: &str, provider_id: &str) -> Result<(), OrderError> {
        let result = sqlx::query(
            "UPDATE orders SET payment_provider = $2, payment_provider_id = $3, updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .bind(provider)
        .bind(provider_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(OrderError::NotFound(id));
        }
        Ok(())
    }

    async fn update_payment_status(&self, id: Uuid, payment_status: &str) -> Result<(), OrderError> {
        let result = sqlx::query(
            "UPDATE orders SET payment_status = $2, updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .bind(payment_status)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(OrderError::NotFound(id));
        }
        Ok(())
    }

    async fn get_all_orders(&self, limit: i64, offset: i64) -> Result<(Vec<Order>, i64), OrderError> {
        let orders = sqlx::query_as::<_, Order>(
            r#"
            SELECT id, user_id, session_id, guest_email, status, payment_status, total_bani,
                   shipping_name, shipping_address, shipping_phone, notes,
                   payment_provider, payment_provider_id, created_at, updated_at
            FROM orders
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar::<_, Option<i64>>("SELECT COUNT(*) FROM orders")
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);

        Ok((orders, total))
    }
}
