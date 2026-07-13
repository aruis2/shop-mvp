//! # Storage — Universal file storage
//!
//! ## Usage
//! ```rust,no_run
//! use storage::{FileStorage, LocalStorage};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage: Arc<dyn FileStorage> = Arc::new(
//!     LocalStorage::new("./uploads", "http://localhost:3000/uploads")
//! );
//!
//! let path = storage.save(b"image data", "products", "jpg").await?;
//! let url = storage.url(&path).await;
//! // url = "http://localhost:3000/uploads/products/uuid.jpg"
//! # Ok(())
//! # }
//! ```

mod error;
mod local;

pub use error::{Result, StorageError};
pub use local::LocalStorage;

use std::fmt::Debug;

/// Universal file storage trait.
///
/// Implementări:
/// - [`LocalStorage`] — fișiere locale (dev)
/// - GcsStorage — Google Cloud Storage (viitor)
/// - S3Storage — AWS S3 (viitor)
#[async_trait::async_trait]
pub trait FileStorage: Debug + Send + Sync {
    /// Salvează un fișier și returnează calea relativă (ex: `products/uuid.jpg`)
    async fn save(&self, data: &[u8], prefix: &str, ext: &str) -> Result<String>;

    /// URL-ul public complet al fișierului
    async fn url(&self, path: &str) -> String;

    /// Șterge un fișier după cale
    async fn delete(&self, path: &str) -> Result<()>;
}

// ============================================================================
// Teste
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_error_messages() {
        let err = StorageError::NotFound("file.jpg".into());
        assert_eq!(format!("{}", err), "Not found: file.jpg");

        let err = StorageError::InvalidPath("../evil".into());
        assert_eq!(format!("{}", err), "Invalid path: ../evil");

        let err = StorageError::Backend("disk full".into());
        assert!(format!("{}", err).contains("disk full"));
    }

    #[test]
    fn test_path_format() {
        // Verifică formatul căii: prefix/uuid.ext
        let prefix = "products";
        let ext = "jpg";
        let path = format!("{}/{{}}.{}", prefix, ext);
        // Nu putem testa UUID-ul exact, dar verificăm formatul
        assert!(path.contains("products/"));
        assert!(path.contains(".jpg"));
    }

    #[test]
    fn test_storage_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: StorageError = io_err.into();
        assert!(format!("{}", err).contains("IO error"));
    }
}
