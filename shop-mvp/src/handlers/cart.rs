// =============================================================================
// 🛒 Cart — capability: doar CartRepo + ProductRepo + RenderService
// =============================================================================

use axum::{
    extract::{Query, State},
    http::HeaderMap,
};
use serde::Deserialize;

use crate::state::CartState;
use crate::render::DetectBasePath;
use crate::handlers::products::render_safe_json;
use crate::boundary::*;
use crate::debug_warn;

fn redirect_back(headers: &axum::http::HeaderMap, fallback: &str, error: Option<&str>) -> SafeResponse {
    crate::boundary::redirect_back(headers, fallback, error)
}

#[derive(Deserialize)]
pub struct CartQuery {
    pub session_id: Option<String>,
    pub error: Option<String>,
    pub added: Option<String>,
}

pub async fn cart_page(
    State(s): State<CartState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<CartQuery>,
) -> SafeResponse {
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
    // Dacă utilizatorul e autentificat, obține coșul și după user_id
    let (cart, user_auth) = if let Some(token) = crate::cookie::get_cookie(
        headers.get("cookie").and_then(|v| v.to_str().ok()).unwrap_or(""), "token"
    ) {
        if let Ok(user) = s.auth.verify_token(token).await {
            match s.cart.get_cart_by_user(&session_id, user.id).await {
                Ok(c) => (c, Some(user.id)),
                Err(e) => return SafeResponse::server_error(e.to_string()),
            }
        } else {
            match s.cart.get_cart(&session_id).await {
                Ok(c) => (c, None),
                Err(e) => return SafeResponse::server_error(e.to_string()),
            }
        }
    } else {
        match s.cart.get_cart(&session_id).await {
            Ok(c) => (c, None),
            Err(e) => return SafeResponse::server_error(e.to_string()),
        }
    };

    let mut total_bani: i64 = 0;
    let mut items_json: Vec<serde_json::Value> = Vec::new();
    let mut private_items_json: Vec<serde_json::Value> = Vec::new();
    let mut public_items_json: Vec<serde_json::Value> = Vec::new();
    let mut private_total_bani: i64 = 0;
    let mut public_total_bani: i64 = 0;
    for item in &cart.items {
        let is_private = item.user_id.is_some();
        let current_price = s.products.get_by_slug(&item.product_slug).await
            .ok()
            .flatten()
            .and_then(|p| p.price_new)
            .map(|p| p as i64)
            .unwrap_or(item.price_bani);
        let subtotal = item.price_bani * item.qty as i64;
        total_bani += subtotal;
        let item_json = serde_json::json!({
            "id": item.id.to_string(), "product_name": item.product_name,
            "product_slug": item.product_slug,
            "price_lei": format!("{:.2}", item.price_bani as f64 / 100.0),
            "price_bani": item.price_bani, "qty": item.qty,
            "is_private": is_private,
            "subtotal_lei": format!("{:.2}", subtotal as f64 / 100.0),
            "current_price_lei": format!("{:.2}", current_price as f64 / 100.0),
        });
        items_json.push(item_json);
        if is_private {
            private_total_bani += subtotal;
            private_items_json.push(serde_json::json!({
                "id": item.id.to_string(), "product_name": item.product_name,
                "product_slug": item.product_slug,
                "price_lei": format!("{:.2}", item.price_bani as f64 / 100.0),
                "qty": item.qty,
                "subtotal_lei": format!("{:.2}", subtotal as f64 / 100.0),
            }));
        } else {
            public_total_bani += subtotal;
            public_items_json.push(serde_json::json!({
                "id": item.id.to_string(), "product_name": item.product_name,
                "product_slug": item.product_slug,
                "price_lei": format!("{:.2}", item.price_bani as f64 / 100.0),
                "qty": item.qty,
                "subtotal_lei": format!("{:.2}", subtotal as f64 / 100.0),
            }));
        }
    }
    let total_lei = format!("{:.2}", total_bani as f64 / 100.0);
    let private_total_lei = format!("{:.2}", private_total_bani as f64 / 100.0);
    let public_total_lei = format!("{:.2}", public_total_bani as f64 / 100.0);

    let mut data = serde_json::json!({
        "title": "Coș de cumpărături — Shop MVP",
        "cart_items": items_json,
        "private_items": private_items_json,
        "public_items": public_items_json,
        "total_lei": total_lei,
        "private_total_lei": private_total_lei,
        "public_total_lei": public_total_lei,
        "item_count": cart.item_count,
        "has_private": !private_items_json.is_empty(),
        "has_public": !public_items_json.is_empty(),
        "is_authenticated": user_auth.is_some(),
        "session_id": session_id,
    });
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    if q.added.is_some() { data["added"] = serde_json::json!("✓ Produs adăugat în coș"); }
    render_safe_json(&s.renderer, "cart/cart.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

// ─── Add to cart ────────────────────────────────────────

pub struct CartAddForm {
    pub product_slug: String,
    pub qty: i32,
}

impl ValidateForm for CartAddForm {
    fn validate(fields: &[FormField], _headers: &HeaderMap) -> Result<Self, SafeResponse> {
        let slug = InputFactory::parse_slug(
            get_field(fields, "product_slug").map_err(|_| SafeResponse::bad_request("Date invalide"))?
        ).map_err(|_| SafeResponse::bad_request("Slug invalid"))?;
        let qty_str = get_field(fields, "qty").unwrap_or("1");
        let qty_val: i32 = qty_str.parse().unwrap_or(1);
        let qty = InputFactory::parse_qty(qty_val)
            .map_err(|_| SafeResponse::bad_request("Cantitate invalidă"))?;
        Ok(CartAddForm { product_slug: slug.as_str().to_string(), qty: qty.get() as i32 })
    }
}

pub async fn cart_add(
    State(s): State<CartState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    ValidatedForm(form): ValidatedForm<CartAddForm>,
) -> SafeResponse {
    let sid = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "session_id"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let product = match s.products.get_by_slug(&form.product_slug).await {
        Ok(Some(p)) => p,
        _ => return redirect_back(&headers, "/products", Some("Produs negăsit")),
    };

    let price_bani = match product.price_new {
        Some(p) => p as i64,
        None => return redirect_back(&headers, "/products", Some("Produsul nu are preț")),
    };

    // 🏭 LogicFactory: validează cantitatea (deja validată de InputFactory, dublă verificare)
    if let Err(_) = LogicFactory::verify_qty_in_range(form.qty, 1, s.max_qty) {
        return redirect_back(&headers, "/products", Some("Cantitate invalidă"));
    }

    // 🏭 LogicFactory: verifică stoc
    if let Err(_) = LogicFactory::verify_stock_available(product.stock_count, form.qty) {
        return redirect_back(&headers, "/products", Some("Stoc insuficient"));
    }

    let req = rust_cart::AddCartItemRequest {
        product_slug: form.product_slug,
        product_name: product.name,
        price_bani,
        qty: form.qty,
    };

    // 🔑 Dacă utilizatorul e autentificat, legăm itemul de user_id
    let token = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "token"));
    let user_id = match token {
        Some(t) => s.auth.verify_token(&t).await.ok().map(|u| u.id),
        None => None,
    };
    if let Err(e) = s.cart.add_item(&sid, user_id, req).await {
        debug_warn!(target: "cart::add", "add_item eșuat: {:?} (user_id={:?})", e, user_id);
        return redirect_back(&headers, "/products", Some("Adăugare eșuată"));
    }

    // 🔁 UX: după adăugare, userul rămâne la produse (nu merge la /cart)
    // - Dacă era pe lista de produse → redirecționează înapoi la listă cu ?added=1
    // - Dacă era pe detaliu produs → redirecționează la /products?added=1
    // - Dacă nu avem referer → fallback la /cart?added=1
    let dest = match headers.get("referer").and_then(|v| v.to_str().ok()) {
        Some(referer) if referer.contains("/product/") => {
            format!("{}/products?added=1", bp)
        }
        Some(referer) => {
            // Extrage doar calea din URL (http://host/path?q= → /path)
            let path = match referer.find("://") {
                Some(pos) => {
                    let after_host = &referer[pos+3..];
                    match after_host.find('/') {
                        Some(slash) => &after_host[slash..],
                        None => "/",
                    }
                }
                None => referer,
            };
            let base = path.split('?').next().unwrap_or(path);
            let safe = OutputFactory::safe_redirect_url(base, "/")
                .unwrap_or_else(|| format!("{}/products", bp));
            format!("{}?added=1", safe)
        }
        None => {
            // 🔗 #cart-container — păstrează poziția scroll după adăugare
            OutputFactory::safe_redirect_url(&format!("{}/cart?added=1#cart-container", bp), "/")
                .unwrap_or_else(|| format!("{}/cart#cart-container", bp))
        }
    };
    SafeResponse::redirect(dest).with_cookie("session_id", &sid, 86400 * 30)
}

// ─── Remove from cart ───────────────────────────────────

/// Formular pentru ștergere item din coș.
/// Validat AUTOMAT de ValidatedForm extractor (V8).
pub struct CartRemoveForm {
    item_id: uuid::Uuid,
}

impl ValidateForm for CartRemoveForm {
    fn validate(fields: &[FormField], _headers: &HeaderMap) -> Result<Self, SafeResponse> {
        let id_str = get_field(fields, "item_id")
            .map_err(|_| SafeResponse::bad_request("Date invalide"))?;
        let item_id = uuid::Uuid::parse_str(id_str)
            .map_err(|_| SafeResponse::bad_request("ID invalid"))?;
        Ok(CartRemoveForm { item_id })
    }
}

pub async fn cart_remove(
    State(s): State<CartState>,
    headers: axum::http::HeaderMap,
    ValidatedForm(form): ValidatedForm<CartRemoveForm>,
) -> SafeResponse {
    let sid = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "session_id"))
        .unwrap_or("anon");

    if let Err(_) = s.cart.remove_item(sid, form.item_id).await {
        return SafeResponse::redirect("/cart?error=Ștergere+eșuată#cart-container");
    }

    // 🔗 #cart-container — browserul duce utilizatorul la secțiunea coș, nu sus
    SafeResponse::redirect("/cart#cart-container")
}

// ─── Update quantity ────────────────────────────────────

/// Formular pentru actualizare cantitate în coș.
/// Validat AUTOMAT de ValidatedForm extractor (V8).
pub struct CartUpdateForm {
    pub item_id: uuid::Uuid,
    pub qty: i32,
}

impl ValidateForm for CartUpdateForm {
    fn validate(fields: &[FormField], _headers: &HeaderMap) -> Result<Self, SafeResponse> {
        let id_str = get_field(fields, "item_id")
            .map_err(|_| SafeResponse::bad_request("Date invalide"))?;
        let item_id = uuid::Uuid::parse_str(id_str)
            .map_err(|_| SafeResponse::bad_request("ID invalid"))?;
        let qty_str = get_field(fields, "qty").unwrap_or("1");
        let qty_val: i32 = qty_str.parse().unwrap_or(1);
        let qty = InputFactory::parse_qty(qty_val)
            .map_err(|_| SafeResponse::bad_request("Cantitate invalidă"))?;
        Ok(CartUpdateForm { item_id, qty: qty.get() as i32 })
    }
}

pub async fn cart_update(
    State(s): State<CartState>,
    headers: axum::http::HeaderMap,
    ValidatedForm(form): ValidatedForm<CartUpdateForm>,
) -> SafeResponse {
    let sid = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "session_id"))
        .unwrap_or("anon");

    // Validează cantitatea
    if let Err(_) = LogicFactory::verify_qty_in_range(form.qty, 1, s.max_qty) {
        return SafeResponse::redirect("/cart?error=Cantitate+invalidă#cart-container");
    }

    if let Err(_) = s.cart.update_qty(sid, form.item_id, form.qty).await {
        return SafeResponse::redirect("/cart?error=Actualizare+eșuată#cart-container");
    }

    // 🔗 #cart-container — browserul duce utilizatorul la secțiunea coș, nu sus
    SafeResponse::redirect("/cart#cart-container")
}
