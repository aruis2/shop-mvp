//! # Cache — Universal cache trait with pluggable backends
//!
//! ## Usage
//! ```rust,no_run
//! use cache::{Cache, PgCache};
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! # async fn example(pool: sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
//! let cache: Arc<dyn Cache> = Arc::new(PgCache::new(pool));
//! cache.init().await?;
//!
//! cache.set("key", "value", Duration::from_secs(60)).await?;
//! let val = cache.get("key").await?;
//! assert_eq!(val, Some("value".to_string()));
//! # Ok(())
//! # }
//! ```

mod error;
mod pg;

pub use error::{CacheError, Result};
pub use pg::PgCache;

use std::fmt::Debug;
use std::time::Duration;

/// Universal cache trait.
///
/// Implementări disponibile:
/// - [`PgCache`] — PostgreSQL (acum)
/// - Redis (viitor)
/// - Memcached (viitor)
///
/// ## `dyn`-compatible
/// Acest trait poate fi folosit ca `Arc<dyn Cache>`.
#[async_trait::async_trait]
pub trait Cache: Debug + Send + Sync {
    /// Inițializează backend-ul (creează tabele, conexiuni etc.).
    async fn init(&self) -> Result<()>;

    /// Obține o valoare din cache după cheie.
    /// Returnează `None` dacă cheia nu există sau a expirat.
    async fn get(&self, key: &str) -> Result<Option<String>>;

    /// Stochează o valoare în cache cu un TTL (durata de viață).
    async fn set(&self, key: &str, value: &str, ttl: Duration) -> Result<()>;

    /// Șterge o cheie din cache.
    async fn delete(&self, key: &str) -> Result<()>;

    /// Verifică dacă o cheie există și nu a expirat.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Golește tot cache-ul.
    async fn flush(&self) -> Result<()>;

    /// Șterge toate cheile expirate.
    async fn clean_expired(&self) -> Result<u64>;
}

// --- Funcții ajutătoare pentru JSON (în afara trait-ului, pentru compatibilitate dyn) ---

/// Stochează un JSON în cache.
pub async fn set_json<T: serde::Serialize + Send + Sync>(
    cache: &dyn Cache,
    key: &str,
    value: &T,
    ttl: Duration,
) -> Result<()> {
    let json = serde_json::to_string(value)?;
    cache.set(key, &json, ttl).await
}

/// Obține și deserializaează un JSON din cache.
pub async fn get_json<T: serde::de::DeserializeOwned>(
    cache: &dyn Cache,
    key: &str,
) -> Result<Option<T>> {
    match cache.get(key).await? {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

// ============================================================================
// Teste
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_set_json_roundtrip() {
        // Verificăm doar logica de serializare/deserializare
        let value = serde_json::json!({"key": "value", "num": 42});
        let json = serde_json::to_string(&value).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["num"], 42);
    }

    #[test]
    fn test_cache_error_display() {
        let err = CacheError::Miss("key not found".into());
        assert_eq!(format!("{}", err), "Cache miss: key not found");
        let err = CacheError::Backend("connection failed".into());
        assert!(format!("{}", err).contains("connection failed"));
    }

    #[test]
    fn test_cache_error_from_serde() {
        let serde_err = serde_json::from_str::<serde_json::Value>("invalid{json").unwrap_err();
        let err: CacheError = serde_err.into();
        // Verificăm doar că e un Serialization error (conține "Serialization")
        assert!(format!("{}", err).contains("Serialization"));
    }

    #[test]
    fn test_ttl_positive() {
        let ttl = Duration::from_secs(60);
        assert!(ttl.as_secs() > 0);
        let ttl2 = Duration::from_millis(500);
        assert!(ttl2.as_millis() > 0);
    }
}
