// =============================================================================
// 🛒 Cart — capability: doar CartRepo + ProductRepo + RenderService
// =============================================================================

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::state::CartState;
use crate::render::DetectBasePath;
use crate::handlers::products::render_or_err_json;
use crate::types::logic::LogicFactory;
use crate::types::output::OutputFactory;
use crate::types::parser::{parse_any_into, get_field};
use crate::types::error::InputError;
use crate::types::InputFactory;
use crate::types::QueryValidator;
use crate::url_encode::url_encode;
use crate::debug_warn;

fn redirect_back(headers: &axum::http::HeaderMap, fallback: &str, error: Option<&str>) -> Response {
    let base = headers.get("referer")
        .and_then(|v| v.to_str().ok())
        .map(|r| r.split('?').next().unwrap_or(r))
        .unwrap_or(fallback);
    if let Some(msg) = error {
        debug_warn!(target: "cart", "redirect_back cu eroare: {} -> {}", msg, base);
    }
    // 🔒 OutputFactory: validează URL-ul redirect (previne open redirect)
    let safe_base = OutputFactory::safe_redirect_url(&base, "/")
        .unwrap_or_else(|| fallback.to_string());
    let dest = match error {
        Some(msg) => format!("{}?error={}", safe_base, url_encode(msg)),
        None => safe_base,
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
    // 🏭 InputFactory: extrage session_id din query → header → cookie
    let session_id = q.session_id.clone()
        .or_else(|| {
            headers.get("x-session-id")
                .and_then(|v| v.to_str().ok().map(String::from))
                .map(|s| QueryValidator::session_id(Some(s), "header:x-session-id")
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()))
        })
        .or_else(|| {
            headers.get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|c| crate::cookie::get_cookie(c, "session_id"))
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let cart = s.cart.get_cart(&session_id).await
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

    let mut data = serde_json::json!({
        "title": "Coș de cumpărături — Shop MVP",
        "cart_items": items_json,
        "total_lei": total_lei,
        "item_count": cart.item_count,
        "session_id": session_id,
    });
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_or_err_json(&s.renderer, "cart/cart.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

// ─── Add to cart ────────────────────────────────────────

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

    // 🏭 InputFactory: parsează și validează chiar la graniță
    let (slug_str, qty) = match parse_any_into(&body, |fields| {
        let slug = InputFactory::parse_slug(get_field(fields, "product_slug")?)?;
        let qty_str = get_field(fields, "qty").unwrap_or("1");
        let qty_val: i32 = qty_str.parse().unwrap_or(1);
        let qty = InputFactory::parse_qty(qty_val)?;
        Ok::<(String, i32), InputError>((slug.as_str().to_string(), qty.get() as i32))
    }) {
        Ok(v) => v,
        Err(InputError::MissingField(_)) | Err(InputError::InvalidSlug(_)) => {
            return redirect_back(&headers, "/products", Some("Date invalide"));
        },
        Err(e) => {
            debug_warn!(target: "cart::add", "InputFactory: {}", e);
            return redirect_back(&headers, "/products", Some("Date invalide"));
        },
    };

    let product = match s.products.get_by_slug(&slug_str).await {
        Ok(Some(p)) => p,
        _ => return redirect_back(&headers, "/products", Some("Produs negăsit")),
    };

    let price_bani = match product.price_new {
        Some(p) => p as i64,
        None => return redirect_back(&headers, "/products", Some("Produsul nu are preț")),
    };

    // 🏭 LogicFactory: validează cantitatea (deja validată de InputFactory, dublă verificare)
    if let Err(_) = LogicFactory::verify_qty_in_range(qty, 1, s.max_qty) {
        return redirect_back(&headers, "/products", Some("Cantitate invalidă"));
    }

    // 🏭 LogicFactory: verifică stoc
    if let Err(_) = LogicFactory::verify_stock_available(product.stock_count, qty) {
        return redirect_back(&headers, "/products", Some("Stoc insuficient"));
    }

    let req = rust_cart::AddCartItemRequest {
        product_slug: slug_str,
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

// ─── Remove from cart ───────────────────────────────────

pub async fn cart_remove(
    State(s): State<CartState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let sid = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "session_id"))
        .unwrap_or("anon");

    // 🏭 InputFactory: parsează item_id
    let item_id_str = match parse_any_into(&body, |fields| {
        let id = get_field(fields, "item_id")?;
        Ok(id.to_string())
    }) {
        Ok(id) => id,
        Err(_) => return redirect_back(&headers, "/cart", Some("Date invalide")),
    };

    let item_id = match uuid::Uuid::parse_str(&item_id_str) {
        Ok(id) => id,
        Err(_) => return redirect_back(&headers, "/cart", Some("ID invalid")),
    };

    if let Err(_) = s.cart.remove_item(sid, item_id).await {
        return redirect_back(&headers, "/cart", Some("Ștergere eșuată"));
    }

    redirect_back(&headers, "/cart", None)
}
