// =============================================================================
// 🛒 Cart — capability: doar CartRepo + ProductRepo + RenderService
// =============================================================================

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use tera::Context;

use crate::state::CartState;
use crate::render::DetectBasePath;
use crate::handlers::products::render_or_err;
use crate::debug_warn;

fn parse_body<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, String> {
    serde_json::from_str::<T>(body)
        .or_else(|_| serde_urlencoded::from_str::<T>(body))
        .map_err(|e| format!("Date invalide: {e}"))
}

fn redirect_back(headers: &axum::http::HeaderMap, fallback: &str, error: Option<&str>) -> Response {
    let base = headers.get("referer")
        .and_then(|v| v.to_str().ok())
        .map(|r| r.split('?').next().unwrap_or(r))
        .unwrap_or(fallback);
    if let Some(msg) = error {
        debug_warn!(target: "cart", "redirect_back cu eroare: {} -> {}", msg, base);
    }
    let dest = match error {
        Some(msg) => format!("{}?error={}", base, msg.replace(' ', "%20")),
        None => base.to_string(),
    };
    (StatusCode::FOUND, [("Location", dest)]).into_response()
}

#[derive(Deserialize)]
pub struct CartQuery {
    pub session_id: Option<String>,
    pub error: Option<String>,
}

pub async fn cart_page(
    State(s): State<CartState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<CartQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let session_id = q.session_id.as_deref()
        .or_else(|| headers.get("x-session-id").and_then(|v| v.to_str().ok()))
        .or_else(|| headers.get("cookie").and_then(|v| v.to_str().ok()).and_then(|c| crate::cookie::get_cookie(c, "session_id")))
        .unwrap_or("anon");
    let cart = s.cart.get_cart(session_id).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut total_bani: i64 = 0;
    let mut items_json: Vec<serde_json::Value> = Vec::new();
    for item in &cart.items {
        let current_price = s.products.get_by_slug(&item.product_slug).await
            .ok()
            .flatten()
            .and_then(|p| p.price_new)
            .map(|p| p as i64)
            .unwrap_or(item.price_bani);
        let subtotal = item.price_bani * item.qty as i64;
        total_bani += subtotal;
        items_json.push(serde_json::json!({
            "id": item.id.to_string(), "product_name": item.product_name,
            "product_slug": item.product_slug,
            "price_lei": format!("{:.2}", item.price_bani as f64 / 100.0),
            "price_bani": item.price_bani, "qty": item.qty,
            "subtotal_lei": format!("{:.2}", subtotal as f64 / 100.0),
            "current_price_lei": format!("{:.2}", current_price as f64 / 100.0),
        }));
    }
    let total_lei = format!("{:.2}", total_bani as f64 / 100.0);

    let mut ctx = Context::new();
    ctx.insert("title", "Coș de cumpărături — Shop MVP");
    ctx.insert("cart_items", &items_json);
    ctx.insert("total_lei", &total_lei);
    ctx.insert("item_count", &cart.item_count);
    ctx.insert("session_id", session_id);
    if let Some(ref e) = q.error { ctx.insert("error", e); }
    render_or_err(&s.renderer, "cart/cart.html", &ctx, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

#[derive(Deserialize)]
pub struct AddItemForm {
    pub product_slug: String,
    pub qty: Option<i32>,
}

pub async fn cart_add(
    State(s): State<CartState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let sid = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "session_id"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let form = match parse_body::<AddItemForm>(&body) {
        Ok(f) => f,
        Err(_) => return redirect_back(&headers, "/products", Some("Date invalide")),
    };

    let product = match s.products.get_by_slug(&form.product_slug).await {
        Ok(Some(p)) => p,
        _ => return redirect_back(&headers, "/products", Some("Produs negăsit")),
    };

    let price_bani = match product.price_new {
        Some(p) => p as i64,
        None => return redirect_back(&headers, "/products", Some("Produsul nu are preț")),
    };

    let qty = form.qty.unwrap_or(1).min(s.max_qty);

    let req = rust_cart::AddCartItemRequest {
        product_slug: form.product_slug,
        product_name: product.name,
        price_bani,
        qty,
    };

    if let Err(_) = s.cart.add_item(&sid, None, req).await {
        return redirect_back(&headers, "/products", Some("Adăugare eșuată"));
    }

    let sid_cookie = crate::cookie::set_cookie("session_id", &sid, 86400 * 30);
    let mut resp = redirect_back(&headers, "/products", None).into_response();
    resp.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&sid_cookie).unwrap(),
    );
    resp
}

#[derive(Deserialize)]
pub struct RemoveItemForm {
    pub item_id: String,
}

pub async fn cart_remove(
    State(s): State<CartState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let sid = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "session_id"))
        .unwrap_or("anon");

    let form = match parse_body::<RemoveItemForm>(&body) {
        Ok(f) => f,
        Err(_) => return redirect_back(&headers, "/cart", Some("Date invalide")),
    };

    let item_id = match uuid::Uuid::parse_str(&form.item_id) {
        Ok(id) => id,
        Err(_) => return redirect_back(&headers, "/cart", Some("ID invalid")),
    };

    if let Err(_) = s.cart.remove_item(sid, item_id).await {
        return redirect_back(&headers, "/cart", Some("Ștergere eșuată"));
    }

    redirect_back(&headers, "/cart", None)
}
