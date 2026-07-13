use crate::{Cache, Result};
use sqlx::PgPool;
use std::fmt;
use std::time::Duration;

/// Implementare PostgreSQL a trait-ului [`Cache`].
///
/// Stochează datele într-o tabelă `cache` cu coloanele:
/// - `key` (TEXT PRIMARY KEY)
/// - `value` (TEXT)
/// - `expires_at` (TIMESTAMPTZ)
#[derive(Clone)]
pub struct PgCache {
    pool: PgPool,
}

impl fmt::Debug for PgCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PgCache").finish()
    }
}

impl PgCache {
    /// Creează un nou cache PostgreSQL.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl Cache for PgCache {
    async fn init(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cache (
                key         TEXT PRIMARY KEY,
                value       TEXT NOT NULL,
                expires_at  TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Șterge intrările expirate la pornire
        self.clean_expired().await?;

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query_scalar::<_, String>(
            r#"
            SELECT value
            FROM cache
            WHERE key = $1 AND expires_at > NOW()
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn set(&self, key: &str, value: &str, ttl: Duration) -> Result<()> {
        let secs = ttl.as_secs() as i64;

        sqlx::query(
            r#"
            INSERT INTO cache (key, value, expires_at)
            VALUES ($1, $2, NOW() + ($3 || ' seconds')::INTERVAL)
            ON CONFLICT (key) DO UPDATE
            SET value = $2, expires_at = NOW() + ($3 || ' seconds')::INTERVAL
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(secs)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM cache WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM cache
                WHERE key = $1 AND expires_at > NOW()
            )
            "#,
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    async fn flush(&self) -> Result<()> {
        sqlx::query("DELETE FROM cache")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn clean_expired(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM cache WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
