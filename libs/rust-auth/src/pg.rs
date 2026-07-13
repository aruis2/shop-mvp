use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;
use crate::{AuthError, AuthRepo};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

/// Implementare PostgreSQL a `AuthRepo`
pub struct PgAuthRepo {
    pool: PgPool,
    jwt_secret: String,
}

impl PgAuthRepo {
    pub fn new(pool: PgPool, jwt_secret: &str) -> Self {
        Self { pool, jwt_secret: jwt_secret.to_string() }
    }

    fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut rand::rngs::OsRng);
        let argon2 = Argon2::default();
        Ok(argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|_| AuthError::InvalidCredentials)?
            .to_string())
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed = PasswordHash::new(hash)
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }

    fn generate_token(&self, user: &User) -> Result<String, AuthError> {
        let now = Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            role: user.role.clone(),
            exp: now + 86400 * 7, // 7 zile
            iat: now,
        };
        // 🔒 HS256 explicit
        let mut header = Header::default();
        header.alg = jsonwebtoken::Algorithm::HS256;
        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|_| AuthError::InvalidToken)
    }
}

#[async_trait]
impl AuthRepo for PgAuthRepo {
    async fn migrate(&self) -> Result<(), AuthError> {
        // Notă: folosim execute() individual pentru fiecare statement,
        // deoarece sqlx+postgres nu suportă multiple statements într-un query()
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'user',
                created_at TIMESTAMPTZ DEFAULT NOW()
            )"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "ALTER TABLE users ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'user'"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE users SET role = 'admin' WHERE email IN ('test@test.org', 'aruis2@gmail.com')"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)"
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn signup(&self, req: CreateUserRequest) -> Result<LoginResponse, AuthError> {
        // Verifică dacă email-ul există deja
        let existing = self.get_by_email(&req.email).await?;
        if existing.is_some() {
            return Err(AuthError::EmailExists);
        }

        let password_hash = self.hash_password(&req.password)?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, name, password_hash, role)
            VALUES ($1, $2, $3, 'user')
            RETURNING id, email, name, password_hash, role, created_at
            "#
        )
        .bind(&req.email)
        .bind(&req.name)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await?;

        let token = self.generate_token(&user)?;
        Ok(LoginResponse {
            token,
            user: user.into(),
        })
    }

    async fn login(&self, req: LoginRequest) -> Result<LoginResponse, AuthError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, name, password_hash, role, created_at
            FROM users
            WHERE email = $1
            "#
        )
        .bind(&req.email)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;

        if !self.verify_password(&req.password, &user.password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }

        let token = self.generate_token(&user)?;
        Ok(LoginResponse {
            token,
            user: user.into(),
        })
    }

    async fn get_user(&self, id: Uuid) -> Result<Option<User>, AuthError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, name, password_hash, role, created_at
            FROM users WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn get_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, name, password_hash, role, created_at
            FROM users WHERE email = $1
            "#
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn verify_token(&self, token: &str) -> Result<User, AuthError> {
        // 🔒 Validare explicită: doar HS256, verificare exp + iss implicit
        let mut validation = Validation::default();
        validation.algorithms = vec![jsonwebtoken::Algorithm::HS256];
        validation.iss = None; // nu verificăm issuer (single-service)

        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken,
        })?;

        let user_id = Uuid::parse_str(&data.claims.sub)
            .map_err(|_| AuthError::InvalidToken)?;

        self.get_user(user_id)
            .await?
            .ok_or(AuthError::UserNotFound)
    }

    async fn list_users(&self) -> Result<Vec<User>, AuthError> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, name, password_hash, role, created_at
            FROM users ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    async fn update_user(&self, id: Uuid, req: UpdateUserRequest) -> Result<Option<User>, AuthError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users SET
                email = COALESCE($1, email),
                name = COALESCE($2, name)
            WHERE id = $3
            RETURNING id, email, name, password_hash, role, created_at
            "#
        )
        .bind(req.email)
        .bind(req.name)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn delete_user(&self, id: Uuid) -> Result<bool, AuthError> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
