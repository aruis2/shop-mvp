// =============================================================================
// 📦 ValidatedForm — Extractor Axum care validează AUTOMAT la graniță
// =============================================================================
// FILOSOFIE: AI-RULES.md Principiul 1 — tot ce intră trece prin INPUT BOUNDARY
//
// În loc ca handler-ele să cheme manual InputFactory::parse_*(),
// acest extractor face validarea AUTOMAT la extracție.
// Handlerul primește date DEJA validate și tipate.
//
// Folosire:
//   pub async fn signup_handler(
//       ValidatedForm(form): ValidatedForm<SignupForm>,
//       ...
//   ) -> SafeResponse
// =============================================================================

use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::StatusCode,
};

use crate::types::parser::{parse_form, FormField};
use crate::boundary::{SafeResponse, SafeStatus};

/// Un formular DEJA validat. Nu poți construi asta fără să treci prin ValidateForm.
/// Asta e SINGURUL mod prin care handler-ele primesc date de la utilizator.
#[derive(Debug)]
pub struct ValidatedForm<T>(pub T);

/// Orice formular care poate fi validat din body URL-encoded.
pub trait ValidateForm: Sized {
    /// Validează cîmpurile și returnează formularul SAU un SafeResponse de eroare.
    fn validate(fields: &[FormField]) -> Result<Self, SafeResponse>;
}

/// Implementare Axum FromRequest — face validarea AUTOMAT la fiecare request.
impl<T, S> FromRequest<S> for ValidatedForm<T>
where
    T: ValidateForm,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let body = Bytes::from_request(req, _state).await
            .map_err(|_| (StatusCode::BAD_REQUEST, "Body error".to_string()))?;
        
        let body_str = String::from_utf8(body.to_vec())
            .map_err(|_| (StatusCode::BAD_REQUEST, "Body invalid".to_string()))?;
        
        let fields = parse_form(&body_str);
        
        match T::validate(&fields) {
            Ok(form) => Ok(ValidatedForm(form)),
            Err(resp) => {
                let status = match resp.status {
                    SafeStatus::BadRequest => StatusCode::BAD_REQUEST,
                    SafeStatus::NotFound => StatusCode::NOT_FOUND,
                    SafeStatus::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
                    SafeStatus::Forbidden => StatusCode::FORBIDDEN,
                    SafeStatus::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
                    _ => StatusCode::BAD_REQUEST,
                };
                Err((status, resp.body))
            }
        }
    }
}
