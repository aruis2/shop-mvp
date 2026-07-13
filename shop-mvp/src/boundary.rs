// =============================================================================
// 🧱 Trust Boundary — UNICA interfață cu exteriorul
// =============================================================================
// FILOSOFIE: TRUST-BOUNDARY.md — tot ce intră/iese trece PRIN AICI
// STANDARD: OWASP ASVS V5.1 (Input), V10 (Output), API Top 10 #1 (BOLA)
//            CIS Control 6 (Access Control)
// =============================================================================
//
// Acest modul reunește TOT ce ține de granița de încredere:
//   - Parsare input (InputFactory, parser, QueryValidator)
//   - Validare header/path/cookie/body (Safe* din rust-trust-boundary)
//   - Reguli business (LogicFactory)
//   - Sanitizare output (OutputFactory, SafeResponse)
//   - Cookie management
//   - Middleware (TrustBoundary + Security Headers)
//   - Front Controller (fallback unic + routing)
//
// Folosire: `use crate::boundary::*;` în loc de 5+ importuri separate.
// =============================================================================

// Re-exporturi din types (Input, Output, Logic, Parser)
pub use crate::types::InputFactory;
pub use crate::types::QueryValidator;
pub use crate::types::error::InputError;
pub use crate::types::logic::{LogicFactory, LogicError};
pub use crate::types::output::OutputFactory;
pub use crate::types::parser::{parse_form, get_field, parse_any_into, FormField};

// Re-exporturi extractor
pub use crate::types::extractor::{ValidatedForm, ValidateForm, redirect_back};

// Re-exporturi cookie
pub use crate::cookie::{get_cookie, set_cookie, remove_cookie};

// Re-exporturi din rust-trust-boundary crate
pub use rust_trust_boundary::{
    SafePath, SafeHeaders, SafeCookies, SafeBody,
    SafeMethod, SafeRequest, SafeResponse, SafeStatus,
    TrustBoundary, BoundaryError,
};

// Re-exporturi middleware + front controller
pub use crate::trust_boundary::{trust_boundary_middleware, SafeRequestPartial};
pub use crate::front_controller::{
    build_inner_router, build_outer_router,
    security_headers_middleware,
};
