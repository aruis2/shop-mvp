// =============================================================================
// 🐛 Debug logging helper — respectă APP_ENV
// =============================================================================
// Folosire:
//   debug_log!(target: "auth", "login eșuat: {}", err);
//   debug_log!("eroare generică");
//
// În debug (APP_ENV=dev sau RUST_LOG=debug) → afișează tot
// În producție → nu afișează nimic (folosește tracing::warn/error pentru aia)

use std::sync::OnceLock;

static IS_DEBUG: OnceLock<bool> = OnceLock::new();

/// Returnează `true` dacă suntem în mod debug.
/// Detectează automat din variabilele de mediu:
/// - `APP_ENV=dev` → debug
/// - `RUST_LOG=debug` → debug
/// - implicit → false (produție)
pub fn is_debug() -> bool {
    *IS_DEBUG.get_or_init(|| {
        let app_env = std::env::var("APP_ENV").unwrap_or_default();
        if app_env == "dev" || app_env == "development" {
            return true;
        }
        let rust_log = std::env::var("RUST_LOG").unwrap_or_default();
        if rust_log.contains("debug") || rust_log == "trace" {
            return true;
        }
        false
    })
}

/// Log only în debug mode (dev).
/// Folosește `tracing::debug!` ca să respecte și `RUST_LOG`.
/// În producție, aceste loguri nu apar (nivelul default e `warn`).
///
/// Dezavantaj: dacă cineva setează `RUST_LOG=debug` în producție, logurile apar.
/// Asta e de fapt un feature — poți activa temporar debugging în producție.
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)+) => {
        tracing::debug!($($arg)+);
    };
}

/// Log only în debug mode, ca warning (vizibil și în producție dacă RUST_LOG=warn).
/// Folosește pentru erori necritice care ajută la debugging.
#[macro_export]
macro_rules! debug_warn {
    ($($arg:tt)+) => {
        if $crate::debug::is_debug() {
            tracing::warn!($($arg)+);
        }
    };
}
