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
    http::HeaderMap,
};

use rust_input_types::{parse_form, FormField};
use crate::boundary::SafeResponse;
use rust_url_normalizer::url_encode;

/// Creează un redirect back la referer cu mesaj de eroare (PRG pattern).
pub fn redirect_back(headers: &HeaderMap, fallback: &str, error: &str) -> SafeResponse {
    let base = headers.get("referer")
        .and_then(|v| v.to_str().ok())
        .map(|r| r.split('?').next().unwrap_or(r))
        .unwrap_or(fallback);
    SafeResponse::redirect(format!("{}?error={}", base, url_encode(error)))
}

/// Un formular DEJA validat. Nu poți construi asta fără să treci prin ValidateForm.
/// Asta e SINGURUL mod prin care handler-ele primesc date de la utilizator.
#[derive(Debug)]
pub struct ValidatedForm<T>(pub T);

/// Orice formular care poate fi validat din body URL-encoded.
/// `validate` primește cîmpurile parșate + headerele HTTP (pentru redirect-uri).
pub trait ValidateForm: Sized {
    /// Validează cîmpurile și returnează formularul.
    /// `headers` — pentru redirect-uri (PRG pattern) care au nevoie de referer.
    /// Pentru erori, returnează un SafeResponse (redirect sau bad_request).
    fn validate(fields: &[FormField], headers: &HeaderMap) -> Result<Self, SafeResponse>;
}

/// Implementare Axum FromRequest — face validarea AUTOMAT la fiecare request.
/// Dacă validarea eșuează, returnează SafeResponse (poate fi redirect sau 400).
impl<T, S> FromRequest<S> for ValidatedForm<T>
where
    T: ValidateForm,
    S: Send + Sync,
{
    type Rejection = SafeResponse;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers().clone();
        
        let body = Bytes::from_request(req, _state).await
            .map_err(|_| SafeResponse::bad_request("Body error"))?;
        
        let body_str = String::from_utf8(body.to_vec())
            .map_err(|_| SafeResponse::bad_request("Body invalid"))?;
        
        let fields = parse_form(&body_str);
        
        T::validate(&fields, &headers)
            .map(ValidatedForm)
    }
}
