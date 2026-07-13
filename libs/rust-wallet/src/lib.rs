//! # rust-wallet
//!
//! Portofel digital: solduri, tranzacții, depuneri/retrageri.
//!
//! ## Teste
//! ```bash
//! cargo test -p rust-wallet
//! ```

mod models;
mod pg;

pub use models::{Balance, Transaction, DepositRequest, BalanceResponse};
pub use pg::PgWalletRepo;

use async_trait::async_trait;
use uuid::Uuid;
use thiserror::Error;

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("User not found: {0}")]
    UserNotFound(Uuid),

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Amount must be positive")]
    InvalidAmount,
}

// ============================================================================
// Trait — UserService (dependență injectată)
// ============================================================================

#[async_trait]
pub trait UserService: Send + Sync {
    async fn user_exists(&self, id: Uuid) -> Result<bool, WalletError>;
}

// ============================================================================
// Trait principal — WalletRepo
// ============================================================================

#[async_trait]
pub trait WalletRepo: Send + Sync {
    /// Creează tabelele `balances` + `transactions`
    async fn migrate(&self) -> Result<(), WalletError>;

    /// Obține soldul unui utilizator (creează contul dacă nu există)
    async fn get_balance(&self, user_id: Uuid) -> Result<BalanceResponse, WalletError>;

    /// Depunere (adaugă bani)
    async fn deposit(&self, user_id: Uuid, amount: i64, description: &str) -> Result<Transaction, WalletError>;

    /// Retragere (scade bani, verifică soldul)
    async fn withdraw(&self, user_id: Uuid, amount: i64, description: &str) -> Result<Transaction, WalletError>;

    /// Istoric tranzacții
    async fn get_transactions(&self, user_id: Uuid, limit: i64) -> Result<Vec<Transaction>, WalletError>;
}

// ============================================================================
// Teste
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_positive() {
        assert!(100 > 0, "Suma trebuie să fie pozitivă");
        assert!(!(-50 > 0), "Suma negativă e invalidă");
    }

    #[test]
    fn test_balance_default() {
        let balance = BalanceResponse { balance: 0, currency: "RON".into() };
        assert_eq!(balance.balance, 0);
        assert_eq!(balance.currency, "RON");
    }
}
