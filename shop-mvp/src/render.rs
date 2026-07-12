// =============================================================================
// 🎨 RenderService — Tera izolat, singurul punct de contact cu template-urile
// =============================================================================
// În spirit seL4: acest serviciu este singurul care are "capability" să
// acceseze template-urile. Orice handler trece prin el — dacă fallback-ează,
// e un singur loc de verificat.

use std::sync::Arc;
use axum::{
    extract::{FromRequestParts, OriginalUri},
    http::request::Parts,
    response::Html,
};
use tera::{Context, Tera};
use crate::types::output::OutputFactory;

/// Extractor care calculează automat base_path din OriginalUri.
/// Funcționează indiferent de adâncimea prefixului:
///   domain/  → ""   (la rădăcină)
///   domain/shop/  → "/shop"
///   domain/magazin/shop/  → "/magazin/shop"
#[derive(Clone, Debug)]
pub struct DetectBasePath(pub String);

impl<S: Send + Sync> FromRequestParts<S> for DetectBasePath {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Dacă e deja în extensions (setat de middleware), returnează direct
        if let Some(bp) = parts.extensions.get::<DetectBasePath>() {
            return Ok(bp.clone());
        }

        // 1. Verifică header-ul X-Forwarded-Prefix (setat de reverse proxy gen Caddy)
        if let Some(prefix) = parts
            .headers
            .get("X-Forwarded-Prefix")
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty())
        {
            let bp = prefix.trim_end_matches('/').to_string();
            parts.extensions.insert(DetectBasePath(bp.clone()));
            return Ok(DetectBasePath(bp));
        }

        // 2. Detectează din OriginalUri (când e folosit .nest() în Axum direct)
        let current = parts.uri.path();
        let original = parts
            .extensions
            .get::<OriginalUri>()
            .map(|u| u.0.path().to_string())
            .unwrap_or_default();

        let base_path = if original.is_empty() || original == current {
            String::new()
        } else if let Some(prefix) = original.strip_suffix(current) {
            prefix.trim_end_matches('/').to_string()
        } else if let Some(prefix) = original.strip_suffix('/') {
            if prefix == current {
                String::new()
            } else {
                prefix.trim_end_matches('/').to_string()
            }
        } else {
            String::new()
        };

        parts.extensions.insert(DetectBasePath(base_path.clone()));
        Ok(DetectBasePath(base_path))
    }
}

#[derive(Clone)]
pub struct RenderService {
    tera: Arc<Tera>,
}

impl RenderService {
    pub fn new(mut tera: Tera) -> Self {
        tera.autoescape_on(vec![".html", ".xml"]);
        Self { tera: Arc::new(tera) }
    }

    /// Redă un template cu un context Tera (clasic).
    /// `base_path` e injectat automat în context.
    /// `partial=true` → doar conținutul; `false` → pagină completă cu layout.
    ///
    /// 🔒 NOTĂ: Acest metodă NU sanitarizează contextul automat.
    ///     Pentru sanitizare automată, folosește `render_json()`.
    pub fn render(&self, template: &str, ctx: &Context, base_path: &str, partial: bool) -> Result<Html<String>, String> {
        let mut ctx = ctx.clone();
        ctx.insert("base_path", base_path);
        self.render_internal(template, ctx, base_path, partial)
    }

    /// Redă un template cu date JSON, cu sanitizare automată OutputFactory.
    /// 🔒 Toate string-urile din `data` trec prin html_encode înainte de Tera.
    /// `base_path` e injectat automat în context.
    /// `partial=true` → doar conținutul; `false` → pagină completă cu layout.
    pub fn render_json(&self, template: &str, data: &serde_json::Value, base_path: &str, partial: bool) -> Result<Html<String>, String> {
        // 🔒 OutputFactory: walk recursiv, html_encode pe toate string-urile
        let sanitized = OutputFactory::sanitize_context(data);
        let ctx = Context::from_serialize(&sanitized)
            .map_err(|e| format!("Context sanitization error: {e}"))?;
        let mut ctx = ctx;
        ctx.insert("base_path", base_path);
        self.render_internal(template, ctx, base_path, partial)
    }

    /// Intern: render propriu-zis + layout wrapping
    fn render_internal(&self, template: &str, ctx: Context, base_path: &str, partial: bool) -> Result<Html<String>, String> {
        let ctx = ctx;

        // Render the content template
        let content = self.tera
            .render(template, &ctx)
            .map_err(|e| format!("Render error în '{template}': {e}"))?;

        if partial {
            Ok(Html(content))
        } else {
            // Full page: încorporăm în layout
            let mut page_ctx = Context::new();
            page_ctx.insert("base_path", base_path);
            page_ctx.insert("content", &content);
            let title = ctx.get("title").and_then(|v| v.as_str()).unwrap_or("Shop MVP");
            page_ctx.insert("title", title);
            page_ctx.insert("head", "");
            if let Some(v) = ctx.get("user_email") { page_ctx.insert("user_email", v); }
            if let Some(v) = ctx.get("user_role") { page_ctx.insert("user_role", v); }
            if let Some(v) = ctx.get("is_admin") { page_ctx.insert("is_admin", v); }
            if let Some(v) = ctx.get("added") { page_ctx.insert("added", v); }
            if let Some(v) = ctx.get("error") { page_ctx.insert("error", v); }

            self.tera
                .render("layout/page.html", &page_ctx)
                .map(Html)
                .map_err(|e| format!("Render error în layout/page.html: {e}"))
        }
    }

    /// Verifică la startup că toate template-urile esențiale există
    pub fn check_templates(&self, required: &[&str]) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for name in required {
            // `render` e single entry point — dacă crapă, știm
            let ctx = Context::new();
            if self.tera.render(name, &ctx).is_err() {
                errors.push(format!("Template lipsă: {name}"));
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    // Pentru teste
    #[cfg(test)]
    pub fn tera(&self) -> &Tera {
        &self.tera
    }
}
