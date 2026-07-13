//! Implementare locală — fișiere pe disc (pentru development)

use crate::{FileStorage, Result};
use std::fmt;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

/// Stochează fișiere local în `base_dir/uploads/`
///
/// URL-urile se servesc via `GET /uploads/{path}`
#[derive(Clone)]
pub struct LocalStorage {
    base_dir: PathBuf,
    base_url: String,
}

impl fmt::Debug for LocalStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalStorage")
            .field("base_dir", &self.base_dir)
            .finish()
    }
}

impl LocalStorage {
    /// Creează un storage local
    ///
    /// `base_dir` = calea pe disc (ex: `./uploads`)
    /// `base_url` = prefix URL (ex: `http://localhost:3000/uploads`)
    pub fn new(base_dir: &str, base_url: &str) -> Self {
        Self {
            base_dir: PathBuf::from(base_dir),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait::async_trait]
impl FileStorage for LocalStorage {
    async fn save(&self, data: &[u8], prefix: &str, ext: &str) -> Result<String> {
        // Generăm un nume unic: {prefix}/{uuid}.{ext}
        let filename = format!("{}/{}.{}", prefix.trim_end_matches('/'), Uuid::new_v4(), ext);
        let full_path = self.base_dir.join(&filename);

        // Crează directoarele dacă nu există
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Scrie fișierul
        fs::write(&full_path, data).await?;

        Ok(filename)
    }

    async fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path)
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.base_dir.join(path);
        fs::remove_file(full_path).await?;
        Ok(())
    }
}
