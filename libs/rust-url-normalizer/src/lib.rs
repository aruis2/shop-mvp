//! # rust-url-normalizer
//!
//! Middleware Axum care normalizează URL-urile:
//! - Elimină slash-ul final (`/produse/` → `/produse`) cu redirect 301
//!
//! Se aplică ca layer global pe orice Router.

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Middleware care redirectează (301) URL-urile cu trailing slash la varianta fără,
/// păstrând query parameters. Rădăcina `/` rămâne neschimbată (e același URL).
pub async fn strip_trailing_slash(
    req: Request,
    next: Next,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();

    // Doar path-uri mai lungi decât "/" au trailing slash în exces
    if path.len() > 1 && path.ends_with('/') {
        let new_path = path.trim_end_matches('/');
        let new_uri = if let Some(query) = req.uri().query() {
            format!("{}?{}", new_path, query)
        } else {
            new_path.to_string()
        };
        let mut resp = Response::new(axum::body::Body::empty());
        *resp.status_mut() = StatusCode::MOVED_PERMANENTLY;
        resp.headers_mut().insert(
            "Location",
            HeaderValue::from_str(&new_uri).unwrap(),
        );
        return resp;
    }

    next.run(req).await.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, middleware, routing::get, Router};
    use http::Request;
    use tower::ServiceExt;

    async fn ok_handler() -> impl IntoResponse {
        (StatusCode::OK, "ok")
    }

    fn app() -> Router {
        Router::new()
            .route("/", get(ok_handler))
            .route("/produse", get(ok_handler))
            .route("/about", get(ok_handler))
            .layer(middleware::from_fn(strip_trailing_slash))
    }

    fn req(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn root_stays_unchanged() {
        let resp = app().oneshot(req("/")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn no_trailing_slash_passes_through() {
        let resp = app().oneshot(req("/produse")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn trailing_slash_redirects_301() {
        let resp = app().oneshot(req("/produse/")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            resp.headers().get("Location").unwrap(),
            "/produse"
        );
    }

    #[tokio::test]
    async fn trailing_slash_preserves_query() {
        let resp = app().oneshot(req("/produse/?page=2")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            resp.headers().get("Location").unwrap(),
            "/produse?page=2"
        );
    }

    #[tokio::test]
    async fn double_slash_strips_all() {
        let resp = app().oneshot(req("/about///")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            resp.headers().get("Location").unwrap(),
            "/about"
        );
    }

    #[tokio::test]
    async fn root_with_query_no_redirect() {
        let resp = app().oneshot(req("/?q=test")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
