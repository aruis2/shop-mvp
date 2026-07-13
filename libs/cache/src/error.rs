use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cache miss: {0}")]
    Miss(String),

    #[error("Backend error: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, CacheError>;
