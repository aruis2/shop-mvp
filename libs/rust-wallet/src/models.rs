use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Soldul unui utilizator
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Balance {
    pub user_id: Uuid,
    pub balance: i64,       // in bani (cents)
    pub currency: String,
    pub updated_at: DateTime<Utc>,
}

/// Răspuns sold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub balance: i64,
    pub currency: String,
}

/// O tranzacție
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub amount: i64,
    pub balance_before: i64,
    pub balance_after: i64,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

/// Request pentru depunere
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositRequest {
    pub amount: i64,
    pub description: Option<String>,
}
