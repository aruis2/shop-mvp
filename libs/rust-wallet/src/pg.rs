use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;
use crate::{BalanceResponse, UserService, WalletError, WalletRepo};

/// Implementare PostgreSQL a `WalletRepo`
pub struct PgWalletRepo {
    pool: PgPool,
    #[allow(dead_code)]
    users: Box<dyn UserService>,
}

impl PgWalletRepo {
    pub fn new(pool: PgPool, users: Box<dyn UserService>) -> Self {
        Self { pool, users }
    }

    /// Creează un cont de wallet pentru un user (dacă nu există)
    async fn ensure_balance(&self, user_id: Uuid) -> Result<Balance, WalletError> {
        let balance = sqlx::query_as::<_, Balance>(
            r#"
            INSERT INTO balances (user_id, balance, currency)
            VALUES ($1, 0, 'RON')
            ON CONFLICT (user_id) DO NOTHING
            RETURNING user_id, balance, currency, updated_at
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(b) = balance {
            Ok(b)
        } else {
            sqlx::query_as::<_, Balance>(
                "SELECT user_id, balance, currency, updated_at FROM balances WHERE user_id = $1"
            )
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(WalletError::from)
        }
    }
}

#[async_trait]
impl WalletRepo for PgWalletRepo {
    async fn migrate(&self) -> Result<(), WalletError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS balances (
                user_id UUID PRIMARY KEY,
                balance BIGINT NOT NULL DEFAULT 0,
                currency TEXT NOT NULL DEFAULT 'RON',
                updated_at TIMESTAMPTZ DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS transactions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL,
                kind TEXT NOT NULL,
                amount BIGINT NOT NULL,
                balance_before BIGINT NOT NULL,
                balance_after BIGINT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                created_at TIMESTAMPTZ DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_transactions_user ON transactions(user_id, created_at DESC);
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_balance(&self, user_id: Uuid) -> Result<BalanceResponse, WalletError> {
        let balance = self.ensure_balance(user_id).await?;
        Ok(BalanceResponse {
            balance: balance.balance,
            currency: balance.currency,
        })
    }

    async fn deposit(&self, user_id: Uuid, amount: i64, description: &str) -> Result<Transaction, WalletError> {
        if amount <= 0 {
            return Err(WalletError::InvalidAmount);
        }

        // Asigură-te că există un rând balance (ignore dacă există deja)
        let _ = self.ensure_balance(user_id).await?;

        let tx = sqlx::query_as::<_, Transaction>(
            r#"
            WITH prev AS (
                SELECT balance FROM balances WHERE user_id = $1
            ),
            updated AS (
                UPDATE balances
                SET balance = balance + $2, updated_at = NOW()
                WHERE user_id = $1
                RETURNING user_id, balance
            )
            INSERT INTO transactions (user_id, kind, amount, balance_before, balance_after, description)
            SELECT $1, 'Deposit', $2, (SELECT balance FROM prev), (SELECT balance FROM updated), $4
            RETURNING id, user_id, kind::text as kind, amount, balance_before, balance_after, description, created_at
            "#
        )
        .bind(user_id)
        .bind(amount)
        .bind(description)
        .fetch_one(&self.pool)
        .await?;

        Ok(tx)
    }

    async fn withdraw(&self, user_id: Uuid, amount: i64, description: &str) -> Result<Transaction, WalletError> {
        if amount <= 0 {
            return Err(WalletError::InvalidAmount);
        }

        let _ = self.ensure_balance(user_id).await?;

        let tx = sqlx::query_as::<_, Transaction>(
            r#"
            WITH prev AS (
                SELECT balance FROM balances WHERE user_id = $1
            ),
            updated AS (
                UPDATE balances
                SET balance = balance - $2, updated_at = NOW()
                WHERE user_id = $1 AND balance >= $2
                RETURNING user_id, balance
            )
            INSERT INTO transactions (user_id, kind, amount, balance_before, balance_after, description)
            SELECT $1, 'Withdraw', $2, (SELECT balance FROM prev), (SELECT balance FROM updated), $4
            WHERE EXISTS (SELECT 1 FROM updated)
            RETURNING id, user_id, kind::text as kind, amount, balance_before, balance_after, description, created_at
            "#
        )
        .bind(user_id)
        .bind(amount)
        .bind(description)
        .fetch_optional(&self.pool)
        .await?;

        tx.ok_or(WalletError::InsufficientBalance)
    }

    async fn get_transactions(&self, user_id: Uuid, limit: i64) -> Result<Vec<Transaction>, WalletError> {
        let txs = sqlx::query_as::<_, Transaction>(
            r#"
            SELECT id, user_id, kind::text as kind, amount,
                   balance_before, balance_after, description, created_at
            FROM transactions
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(txs)
    }
}
