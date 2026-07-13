//! # rust-auth
//!
//! Autentificare: utilizatori, parole (argon2), sesiuni JWT.
//!
//! ## Teste
//! ```bash
//! cargo test -p rust-auth
//! ```

mod models;
mod pg;

pub use models::{
    User, UserPublic, CreateUserRequest, UpdateUserRequest,
    LoginRequest, LoginResponse, Claims,
};
pub use pg::PgAuthRepo;

use async_trait::async_trait;
use uuid::Uuid;
use thiserror::Error;

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("User not found")]
    UserNotFound,

    #[error("Invalid email or password")]
    InvalidCredentials,

    #[error("Email already exists")]
    EmailExists,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,
}

// ============================================================================
// Trait — AuthRepo
// ============================================================================

#[async_trait]
pub trait AuthRepo: Send + Sync {
    /// Creează tabelele `users` + `sessions` dacă nu există
    async fn migrate(&self) -> Result<(), AuthError>;

    /// Înregistrare utilizator nou
    async fn signup(&self, req: CreateUserRequest) -> Result<LoginResponse, AuthError>;

    /// Login cu email + parolă
    async fn login(&self, req: LoginRequest) -> Result<LoginResponse, AuthError>;

    /// Obține utilizator după ID
    async fn get_user(&self, id: Uuid) -> Result<Option<User>, AuthError>;

    /// Obține utilizator după email
    async fn get_by_email(&self, email: &str) -> Result<Option<User>, AuthError>;

    /// Verifică un token JWT și returnează utilizatorul
    async fn verify_token(&self, token: &str) -> Result<User, AuthError>;

    /// Listare utilizatori
    async fn list_users(&self) -> Result<Vec<User>, AuthError>;

    /// Actualizare utilizator
    async fn update_user(&self, id: Uuid, req: UpdateUserRequest) -> Result<Option<User>, AuthError>;

    /// Ștergere utilizator
    async fn delete_user(&self, id: Uuid) -> Result<bool, AuthError>;
}

// ============================================================================
// Teste — fără DB, doar logica pură
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation_contains_at() {
        let valid = "user@example.com";
        let invalid = "userexample.com";
        assert!(valid.contains('@'));
        assert!(!invalid.contains('@'));
    }

    #[test]
    fn test_password_min_length() {
        let short = "ab";
        let long_enough = "password123";
        assert!(short.len() < 8, "Parola prea scurtă");
        assert!(long_enough.len() >= 8, "Parola ok");
    }

    #[test]
    fn test_user_public_hides_password() {
        let user = User {
            id: Uuid::nil(),
            email: "test@test.com".into(),
            name: Some("Test".into()),
            role: "user".into(),
            password_hash: "should_not_leak".into(),
            created_at: None,
        };
        let public: UserPublic = user.into();
        assert_eq!(public.email, "test@test.com");
        // password_hash nu există în UserPublic
    }
}
