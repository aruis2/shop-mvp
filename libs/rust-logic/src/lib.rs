// =============================================================================
// 🏭 LogicFactory — Uzina de logică business (reguli de domeniu)
// =============================================================================
// FILOSOFIE: PHILOSOPHY #15 (Rust gap — ce nu prinde compilatorul)
// STANDARD: OWASP ASVS V2.8 (IDOR), V4.2 (State machine), V2 (Authorization)
//          OWASP API Top 10 #1 (Broken Object Level Auth)
// =============================================================================
//
// InputFactory parsează datele (sintaxă).
// OutputFactory sanitarizează ieșirea (XSS, open redirect).
// LogicFactory verifică REGULILE DE BUSINESS (semantică):
//   - Cine deține acest obiect? (IDOR)
//   - Are voie utilizatorul să facă asta? (authorization)
//   - E permisă această tranziție de stare? (state machine)
//   - E stoc suficient? (business rules)
//
// Rust prinde ~70% din bug-uri la compilare (memorie, tipuri, null).
// LogicFactory prinde ~20% din ce rămîne (reguli de business).
// Restul (~10%) — unwrap, overflow, deadlock — se rezolvă cu
//   practici de cod (?, .get(), checked_add()) și tooling (clippy, loom).
// =============================================================================

use std::fmt;

/// Erori de business logic — returnate de LogicFactory
#[derive(Debug, Clone, PartialEq)]
pub enum LogicError {
    /// IDOR: obiectul nu aparține userului
    Forbidden,
    /// Rol insuficient (nu e admin)
    Unauthorized(String),
    /// Tranziție de stare invalidă
    InvalidStatus(String),
    /// Stoc insuficient
    InsufficientStock(i32, i32),
    /// Depășire limită
    LimitExceeded(String),
    /// Resursă negăsită
    NotFound(String),
    /// Operație duplicat
    Duplicate(String),
    /// Altă eroare de business
    Other(String),
}

impl fmt::Display for LogicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicError::Forbidden => write!(f, "Acces interzis"),
            LogicError::Unauthorized(role) => write!(f, "Rol insuficient: {}", role),
            LogicError::InvalidStatus(s) => write!(f, "Stare invalidă: {}", s),
            LogicError::InsufficientStock(have, want) => {
                write!(f, "Stoc insuficient: {} disponibil, {} cerut", have, want)
            }
            LogicError::LimitExceeded(msg) => write!(f, "Limită depășită: {}", msg),
            LogicError::NotFound(msg) => write!(f, "Negăsit: {}", msg),
            LogicError::Duplicate(msg) => write!(f, "Duplicat: {}", msg),
            LogicError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for LogicError {}

/// LogicFactory — verificări de business logic.
/// Toate metodele returnează `Result<(), LogicError>`.
pub struct LogicFactory;

impl LogicFactory {
    // ─── Ownership (IDOR) ────────────────────────────────

    /// Verifică că `user_id` e proprietarul lui `owner_id`.
    /// Previne IDOR (Insecure Direct Object Reference).
    pub fn verify_ownership<T: Eq + fmt::Debug>(
        user_id: &T,
        owner_id: &T,
        object_name: &str,
    ) -> Result<(), LogicError> {
        if user_id == owner_id {
            Ok(())
        } else {
            tracing::warn!(target: "logic::idor",
                "IDOR încercat: user={:?} owner={:?} object={}",
                user_id, owner_id, object_name);
            Err(LogicError::Forbidden)
        }
    }

    // ─── Authorization ───────────────────────────────────

    /// Verifică că userul are rol de admin.
    pub fn verify_admin(role: &str) -> Result<(), LogicError> {
        if role == "admin" {
            Ok(())
        } else {
            Err(LogicError::Unauthorized("admin".to_string()))
        }
    }

    /// Verifică că userul are un rol specific.
    pub fn verify_role(role: &str, required: &str) -> Result<(), LogicError> {
        if role == required {
            Ok(())
        } else {
            Err(LogicError::Unauthorized(required.to_string()))
        }
    }

    // ─── State machine ───────────────────────────────────

    /// Verifică că plata nu a fost deja făcută.
    pub fn verify_not_paid(payment_status: &str) -> Result<(), LogicError> {
        if payment_status == "paid" {
            Err(LogicError::InvalidStatus("Comanda e deja plătită".to_string()))
        } else {
            Ok(())
        }
    }

    /// Verifică că o tranziție de stare e validă.
    /// `current` → stare curentă, `next` → stare dorită.
    pub fn verify_status_transition(current: &str, next: &str) -> Result<(), LogicError> {
        let valid = matches!(
            (current, next),
            ("pending", "confirmed")
                | ("pending", "cancelled")
                | ("confirmed", "shipped")
                | ("confirmed", "cancelled")
                | ("shipped", "delivered")
                | ("paid", "refunded")
                | ("unpaid", "paid")
        );
        if valid {
            Ok(())
        } else {
            Err(LogicError::InvalidStatus(
                format!("Tranziția {current} → {next} nu e permisă")
            ))
        }
    }

    // ─── Business rules ──────────────────────────────────

    /// Verifică că stocul e suficient.
    pub fn verify_stock_available(stock: i32, requested: i32) -> Result<(), LogicError> {
        if stock >= requested {
            Ok(())
        } else {
            Err(LogicError::InsufficientStock(stock, requested))
        }
    }

    /// Verifică că o cantitate e în limite.
    pub fn verify_qty_in_range(qty: i32, min: i32, max: i32) -> Result<(), LogicError> {
        if qty >= min && qty <= max {
            Ok(())
        } else {
            Err(LogicError::LimitExceeded(
                format!("Cantitatea {qty} nu e în intervalul [{min}, {max}]")
            ))
        }
    }

    /// Verifică că valoarea totală nu depășește maximul.
    pub fn verify_max_value(value: i64, max: i64, label: &str) -> Result<(), LogicError> {
        if value <= max {
            Ok(())
        } else {
            Err(LogicError::LimitExceeded(
                format!("{label} {value} depășește maximul {max}")
            ))
        }
    }

    /// Verifică că resursa există (negăsită → eroare).
    pub fn verify_found<T>(resource: Option<T>, name: &str) -> Result<T, LogicError> {
        resource.ok_or_else(|| LogicError::NotFound(name.to_string()))
    }

    /// Verifică idempotency — operatia n-ar trebui să fie deja executată.
    pub fn verify_not_duplicate(already_done: bool, msg: &str) -> Result<(), LogicError> {
        if already_done {
            Err(LogicError::Duplicate(msg.to_string()))
        } else {
            Ok(())
        }
    }
}

// =============================================================================
// Teste
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Ownership ───────────────────────────────────────

    #[test]
    fn ownership_matches() {
        assert!(LogicFactory::verify_ownership(&1, &1, "order").is_ok());
    }

    #[test]
    fn ownership_mismatch() {
        assert_eq!(
            LogicFactory::verify_ownership(&1, &2, "order"),
            Err(LogicError::Forbidden)
        );
    }

    // ─── Authorization ───────────────────────────────────

    #[test]
    fn admin_allowed() {
        assert!(LogicFactory::verify_admin("admin").is_ok());
    }

    #[test]
    fn admin_denied() {
        assert_eq!(
            LogicFactory::verify_admin("user"),
            Err(LogicError::Unauthorized("admin".to_string()))
        );
    }

    #[test]
    fn role_allowed() {
        assert!(LogicFactory::verify_role("admin", "admin").is_ok());
    }

    #[test]
    fn role_denied() {
        assert_eq!(
            LogicFactory::verify_role("user", "admin"),
            Err(LogicError::Unauthorized("admin".to_string()))
        );
    }

    // ─── State machine ───────────────────────────────────

    #[test]
    fn not_paid_allowed() {
        assert!(LogicFactory::verify_not_paid("unpaid").is_ok());
        assert!(LogicFactory::verify_not_paid("pending").is_ok());
    }

    #[test]
    fn not_paid_denied() {
        assert_eq!(
            LogicFactory::verify_not_paid("paid"),
            Err(LogicError::InvalidStatus("Comanda e deja plătită".to_string()))
        );
    }

    #[test]
    fn valid_transitions() {
        assert!(LogicFactory::verify_status_transition("pending", "confirmed").is_ok());
        assert!(LogicFactory::verify_status_transition("pending", "cancelled").is_ok());
        assert!(LogicFactory::verify_status_transition("confirmed", "shipped").is_ok());
        assert!(LogicFactory::verify_status_transition("shipped", "delivered").is_ok());
        assert!(LogicFactory::verify_status_transition("paid", "refunded").is_ok());
    }

    #[test]
    fn invalid_transition() {
        assert_eq!(
            LogicFactory::verify_status_transition("delivered", "confirmed"),
            Err(LogicError::InvalidStatus(
                "Tranziția delivered → confirmed nu e permisă".to_string()
            ))
        );
    }

    // ─── Business rules ──────────────────────────────────

    #[test]
    fn stock_sufficient() {
        assert!(LogicFactory::verify_stock_available(10, 5).is_ok());
    }

    #[test]
    fn stock_insufficient() {
        assert_eq!(
            LogicFactory::verify_stock_available(3, 5),
            Err(LogicError::InsufficientStock(3, 5))
        );
    }

    #[test]
    fn qty_in_range() {
        assert!(LogicFactory::verify_qty_in_range(5, 1, 10).is_ok());
    }

    #[test]
    fn qty_out_of_range() {
        assert_eq!(
            LogicFactory::verify_qty_in_range(0, 1, 10),
            Err(LogicError::LimitExceeded(
                "Cantitatea 0 nu e în intervalul [1, 10]".to_string()
            ))
        );
    }

    #[test]
    fn max_value_ok() {
        assert!(LogicFactory::verify_max_value(100, 200, "total").is_ok());
    }

    #[test]
    fn max_value_exceeded() {
        assert_eq!(
            LogicFactory::verify_max_value(300, 200, "total"),
            Err(LogicError::LimitExceeded(
                "total 300 depășește maximul 200".to_string()
            ))
        );
    }

    #[test]
    fn found_some() {
        let result: Result<i32, LogicError> = LogicFactory::verify_found(Some(42), "item");
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn found_none() {
        let result: Result<i32, LogicError> = LogicFactory::verify_found(None, "item");
        assert_eq!(result, Err(LogicError::NotFound("item".to_string())));
    }

    #[test]
    fn not_duplicate() {
        assert!(LogicFactory::verify_not_duplicate(false, "deja există").is_ok());
    }

    #[test]
    fn duplicate() {
        assert_eq!(
            LogicFactory::verify_not_duplicate(true, "deja există"),
            Err(LogicError::Duplicate("deja există".to_string()))
        );
    }
}
