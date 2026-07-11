// =============================================================================
// 📦 Products — capability: doar ProductRepo + RenderService
// =============================================================================

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Html,
};
use serde::Deserialize;
use tera::Context;
use tracing;

use crate::state::ProductState;
use crate::render::{RenderService, DetectBasePath};
use crate::handlers::auth;
use crate::types::InputFactory;

pub const PRODUCTS_PER_PAGE: i64 = 24;

/// Helper: render cu mapare de eroare + injectare user (Context clasic)
pub async fn render_or_err(
    renderer: &RenderService,
    template: &str,
    ctx: &Context,
    base_path: &str,
    is_htmx: bool,
    headers: &HeaderMap,
    auth_repo: &dyn rust_auth::AuthRepo,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let mut ctx = ctx.clone();
    auth::inject_user_ctx(&mut ctx, headers, auth_repo).await;
    renderer.render(template, &ctx, base_path, is_htmx)
        .map_err(|e| {
            tracing::error!(target: "template", "Eșec render '{}': {}", template, e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)
        })
}

/// 🔒 Helper: render cu sanitizare OutputFactory automată (serde_json::Value).
/// Folosește render_json() care html_encode pe toate string-urile înainte de Tera.
pub async fn render_or_err_json(
    renderer: &RenderService,
    template: &str,
    data: &serde_json::Value,
    base_path: &str,
    is_htmx: bool,
    headers: &HeaderMap,
    auth_repo: &dyn rust_auth::AuthRepo,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let mut data = data.clone();
    auth::inject_user_ctx_json(&mut data, headers, auth_repo).await;
    renderer.render_json(template, &data, base_path, is_htmx)
        .map_err(|e| {
            tracing::error!(target: "template", "Eșec render '{}': {}", template, e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)
        })
}

#[derive(Deserialize, Default)]
pub struct ProductsQuery {
    pub page: Option<i64>,
    pub category: Option<i32>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub page: Option<i64>,
}

async fn fetch_categories(db: &sqlx::PgPool) -> Vec<serde_json::Value> {
    sqlx::query_as::<_, (i32, String, String)>("SELECT id, name, slug FROM categories ORDER BY name")
        .fetch_all(db)
        .await
        .unwrap_or_default()
        .iter()
        .map(|(id, name, slug)| serde_json::json!({"id": id, "name": name, "slug": slug}))
        .collect()
}

#[derive(Deserialize)]
pub struct HomeQuery {
    pub error: Option<String>,
}

pub async fn home_page(
    State(s): State<ProductState>,
    DetectBasePath(bp): DetectBasePath,
    headers: HeaderMap,
    Query(q): Query<HomeQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let mut data = serde_json::json!({"title": "Acasă — Shop MVP"});
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_or_err_json(&s.renderer, "index.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn search_page(
    State(s): State<ProductState>,
    DetectBasePath(bp): DetectBasePath,
    headers: HeaderMap,
    Query(q): Query<SearchQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    // 🏭 InputFactory: validează query-ul de căutare (permitem și gol)
    let query_str = if q.q.is_empty() {
        String::new()
    } else {
        match InputFactory::parse_search(&q.q) {
            Ok(sq) => sq.as_str().to_string(),
            Err(_) => return Err((StatusCode::BAD_REQUEST, "Query invalid".to_string())),
        }
    };
    if query_str.is_empty() {
        let data = serde_json::json!({
            "title": "Căutare — Shop MVP",
            "products": [],
            "total": 0,
            "page": 1,
            "total_pages": 1,
            "query": "",
        });
        return render_or_err_json(&s.renderer, "products/search.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await;
    }
    let page = q.page.unwrap_or(1).max(1);
    let (products, total) = s.products.search_products(&query_str, page, PRODUCTS_PER_PAGE)
        .await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let products_json: Vec<serde_json::Value> = products.iter().map(|p| {
        let price_lei = p.price_new.map(|v| format!("{:.2}", v as f64 / 100.0));
        serde_json::json!({
            "id": p.id, "brand": p.brand, "name": p.name, "slug": p.slug,
            "price_new": p.price_new, "price_lei": price_lei, "image_url": p.image_url,
            "stock_count": p.stock_count,
        })
    }).collect();

    let total_pages = ((total as f64) / PRODUCTS_PER_PAGE as f64).ceil() as i64;

    let data = serde_json::json!({
        "title": format!("Căutare: {} — Shop MVP", query_str),
        "products": products_json,
        "total": total,
        "page": page,
        "total_pages": total_pages,
        "query": query_str,
    });
    render_or_err_json(&s.renderer, "products/search.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn products_page(
    State(s): State<ProductState>,
    DetectBasePath(bp): DetectBasePath,
    headers: HeaderMap,
    Query(q): Query<ProductsQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let page = q.page.unwrap_or(1).max(1);
    let cat_id = q.category;
    let (products, _total) = s.products.get_products(None, page, PRODUCTS_PER_PAGE)
        .await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Filtrare pe categorii client-side (simplu, fără modificare trait)
    let products: Vec<_> = if let Some(cid) = cat_id {
        products.into_iter().filter(|p| p.category_id == cid).collect()
    } else {
        products
    };
    let total = products.len() as i64;

    let products_json: Vec<serde_json::Value> = products.iter().map(|p| {
        let price_lei = p.price_new.map(|v| format!("{:.2}", v as f64 / 100.0));
        serde_json::json!({
            "id": p.id, "brand": p.brand, "name": p.name, "slug": p.slug,
            "price_new": p.price_new, "price_lei": price_lei, "image_url": p.image_url,
            "stock_count": p.stock_count,
        })
    }).collect();

    let total_pages = ((total as f64) / PRODUCTS_PER_PAGE as f64).ceil() as i64;
    let categories = fetch_categories(&s.db).await;

    let data = serde_json::json!({
        "title": "📦 Produse",
        "products": products_json,
        "categories": categories,
        "category_id": cat_id,
        "total": total,
        "page": page,
        "total_pages": total_pages,
    });
    render_or_err_json(&s.renderer, "products/products.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn product_detail_page(
    State(s): State<ProductState>,
    DetectBasePath(bp): DetectBasePath,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let product = s.products.get_by_slug(&slug).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((axum::http::StatusCode::NOT_FOUND, "Produs negăsit".into()))?;

    let price_lei = product.price_new.map(|v| format!("{:.2}", v as f64 / 100.0));
    let specs_arr: Vec<serde_json::Value> = product.specs.as_object().map(|m| {
        m.iter().map(|(k, v)| {
            let val = v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())).unwrap_or_default();
            serde_json::json!({"key": k, "value": val})
        }).collect()
    }).unwrap_or_default();
    let data = serde_json::json!({
        "title": format!("{} — Shop MVP", product.name),
        "product": {
            "id": product.id, "brand": product.brand, "name": product.name,
            "slug": product.slug, "price_new": product.price_new,
            "price_lei": price_lei, "image_url": product.image_url,
            "specs": specs_arr, "stock_count": product.stock_count,
        },
    });
    render_or_err_json(&s.renderer, "products/product_detail.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}
