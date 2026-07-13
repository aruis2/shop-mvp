// =============================================================================
// 📦 Products — capability: doar ProductRepo + RenderService
// =============================================================================

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Html,
};
use serde::Deserialize;
use tera::Context;
use tracing;

use crate::state::ProductState;
use crate::render::{RenderService, DetectBasePath};
use crate::handlers::auth;
use crate::boundary::*;

pub const PRODUCTS_PER_PAGE: i64 = 24;

/// Helper: render cu mapare de eroare + injectare user (Context clasic)
pub async fn render_or_err(
    renderer: &RenderService,
    template: &str,
    ctx: &Context,
    base_path: &str,
    headers: &HeaderMap,
    auth_repo: &dyn rust_auth::AuthRepo,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let mut ctx = ctx.clone();
    auth::inject_user_ctx(&mut ctx, headers, auth_repo).await;
    renderer.render(template, &ctx, base_path, false)
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
    headers: &HeaderMap,
    auth_repo: &dyn rust_auth::AuthRepo,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let mut data = data.clone();
    auth::inject_user_ctx_json(&mut data, headers, auth_repo).await;
    renderer.render_json(template, &data, base_path, false)
        .map_err(|e| {
            tracing::error!(target: "template", "Eșec render '{}': {}", template, e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)
        })
}

/// ✅ V7: Versiunea SafeResponse a render_or_err_json.
/// Returnează SafeResponse direct — output garantat sigur.
pub async fn render_safe_json(
    renderer: &RenderService,
    template: &str,
    data: &serde_json::Value,
    base_path: &str,
    headers: &HeaderMap,
    auth_repo: &dyn rust_auth::AuthRepo,
) -> SafeResponse {
    match render_or_err_json(renderer, template, data, base_path, headers, auth_repo).await {
        Ok(html) => SafeResponse::html(html.0),
        Err((_code, msg)) => SafeResponse::server_error(msg),
    }
}

/// ✅ V7: Versiunea SafeResponse a render_or_err.
pub async fn render_safe(
    renderer: &RenderService,
    template: &str,
    ctx: &Context,
    base_path: &str,
    headers: &HeaderMap,
    auth_repo: &dyn rust_auth::AuthRepo,
) -> SafeResponse {
    match render_or_err(renderer, template, ctx, base_path, headers, auth_repo).await {
        Ok(html) => SafeResponse::html(html.0),
        Err((_code, msg)) => SafeResponse::server_error(msg),
    }
}

#[derive(Deserialize, Default)]
pub struct ProductsQuery {
    pub page: Option<i64>,
    pub category: Option<i32>,
    pub added: Option<String>,
    pub error: Option<String>,
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
) -> SafeResponse {
    let mut data = serde_json::json!({"title": "Acasă — Shop MVP"});
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_safe_json(&s.renderer, "index.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn search_page(
    State(s): State<ProductState>,
    DetectBasePath(bp): DetectBasePath,
    headers: HeaderMap,
    Query(q): Query<SearchQuery>,
) -> SafeResponse {
    // 🏭 InputFactory: validează query-ul de căutare (permitem și gol)
    let query_str = if q.q.is_empty() {
        String::new()
    } else {
        match InputFactory::parse_search(&q.q) {
            Ok(sq) => sq.as_str().to_string(),
            Err(_) => return SafeResponse::bad_request("Query invalid"),
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
        return render_safe_json(&s.renderer, "products/search.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await;
    }
    let page = QueryValidator::page(q.page, 1);
    let (products, total) = match s.products.search_products(&query_str, page, PRODUCTS_PER_PAGE).await {
        Ok(v) => v,
        Err(e) => return SafeResponse::server_error(e.to_string()),
    };

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
    render_safe_json(&s.renderer, "products/search.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn products_page(
    State(s): State<ProductState>,
    DetectBasePath(bp): DetectBasePath,
    headers: HeaderMap,
    Query(q): Query<ProductsQuery>,
) -> SafeResponse {
    let page = QueryValidator::page(q.page, 1);
    let cat_id = q.category;
    let (all_products, db_total) = match s.products.get_products(None, page, PRODUCTS_PER_PAGE).await {
        Ok(v) => v,
        Err(e) => return SafeResponse::server_error(e.to_string()),
    };

    // Filtrare pe categorii client-side (simplu, fără modificare trait)
    let products: Vec<_> = if let Some(cid) = cat_id {
        all_products.into_iter().filter(|p| p.category_id == cid).collect()
    } else {
        all_products
    };
    // 🔒 Folosim db_total (numărul real din DB), nu products.len() (care e cel mult o pagină)
    let total = db_total;

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

    let mut data = serde_json::json!({
        "title": "📦 Produse",
        "products": products_json,
        "categories": categories,
        "category_id": cat_id,
        "total": total,
        "page": page,
        "total_pages": total_pages,
    });
    if q.added.is_some() { data["added"] = serde_json::json!("✓ Produs adăugat în coș"); }
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_safe_json(&s.renderer, "products/products.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

#[derive(Deserialize)]
pub struct DetailQuery {
    pub added: Option<String>,
    pub error: Option<String>,
}

pub async fn product_detail_page(
    State(s): State<ProductState>,
    DetectBasePath(bp): DetectBasePath,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Query(q): Query<DetailQuery>,
) -> SafeResponse {
    let product = match s.products.get_by_slug(&slug).await {
        Ok(Some(p)) => p,
        Ok(None) => return SafeResponse::not_found(),
        Err(e) => return SafeResponse::server_error(e.to_string()),
    };

    let price_lei = product.price_new.map(|v| format!("{:.2}", v as f64 / 100.0));
    let specs_arr: Vec<serde_json::Value> = product.specs.as_object().map(|m| {
        m.iter().map(|(k, v)| {
            let val = v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())).unwrap_or_default();
            serde_json::json!({"key": k, "value": val})
        }).collect()
    }).unwrap_or_default();
    let mut data = serde_json::json!({
        "title": format!("{} — Shop MVP", product.name),
        "product": {
            "id": product.id, "brand": product.brand, "name": product.name,
            "slug": product.slug, "price_new": product.price_new,
            "price_lei": price_lei, "image_url": product.image_url,
            "specs": specs_arr, "stock_count": product.stock_count,
        },
    });
    if q.added.is_some() { data["added"] = serde_json::json!("✓ Produs adăugat în coș"); }
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_safe_json(&s.renderer, "products/product_detail.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}
