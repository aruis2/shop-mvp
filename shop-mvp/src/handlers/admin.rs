// =============================================================================
// 🔐 Admin — capability: ProductsRepo + OrderRepo + PaymentRepo + Auth
// =============================================================================
// Cel mai puternic domain, dar tot prin trait-uri, nu PgPool direct.
// Chiar și admin_migrate_orders merge prin OrderRepo, nu query direct.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::state::AdminState;
use crate::render::DetectBasePath;
use crate::handlers::products::render_or_err_json;
use crate::boundary::*;
use crate::types::parser::{parse_any_into, get_field};
use crate::url_encode::url_encode;
use crate::debug_warn;

// ─── Helper ────────────────────────────────────────────────────

async fn verify_admin(
    headers: &axum::http::HeaderMap,
    q: &AdminQuery,
    auth: &dyn rust_auth::AuthRepo,
) -> Result<rust_auth::User, (axum::http::StatusCode, String)> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| q.token.as_deref())
        .or_else(|| {
            headers.get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|c| crate::cookie::get_cookie(c, "token"))
        })
        .ok_or((axum::http::StatusCode::UNAUTHORIZED, "Admin: token lipsă".into()))?;
    let user = auth.verify_token(token).await
        .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "Admin: token invalid".into()))?;
    LogicFactory::verify_admin(&user.role).map_err(|_| {
        (axum::http::StatusCode::FORBIDDEN, "Admin: acces interzis".into())
    })?;
    Ok(user)
}

fn render_admin_redirect(base_path: &str, redirect_path: &str) -> Html<String> {
    let bp = OutputFactory::text_html(base_path);
    let rp = OutputFactory::text_html(redirect_path);
    // 🔒 Fără inline script (blocat de CSP script-src 'self').
    // Meta refresh e 100% HTML, nu JS — respectă CSP.
    let url = format!("{}/login?redirect={}", bp, rp);
    let safe_url = OutputFactory::text_html(&url);
    Html(format!(r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0;url={safe_url}"></head><body><p><a href="{safe_url}">Continuă</a></p></body></html>"#))
}

async fn verify_or_redirect(
    headers: &axum::http::HeaderMap,
    q: &AdminQuery,
    auth: &dyn rust_auth::AuthRepo,
    bp: &str,
    redirect_path: &str,
) -> Result<rust_auth::User, Html<String>> {
    let rp = if redirect_path.is_empty() { format!("{}/admin", bp) } else { redirect_path.to_string() };
    match verify_admin(headers, q, auth).await {
        Ok(user) => Ok(user),
        Err((status, msg)) => {
            debug_warn!(target: "admin::verify", "verify_admin eșuat: {} {} path={}", status, msg, rp);
            if status == axum::http::StatusCode::FORBIDDEN {
                // Autentificat dar nu e admin → redirect la home, nu la login
                let dest = format!("{}/?error={}", bp, url_encode(&msg));
                let safe_dest = OutputFactory::text_html(&dest);
                Err(Html(format!(r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0;url={safe_dest}"></head><body><p><a href="{safe_dest}">Continuă</a></p></body></html>"#)))
            } else {
                // Neautentificat → redirect la login
                Err(render_admin_redirect(bp, &rp))
            }
        }
    }
}

// ─── Tipuri comune ─────────────────────────────────────────────

const ADMIN_PER_PAGE: i64 = 25;

#[derive(Deserialize)]
pub struct AdminQuery {
    pub token: Option<String>,
    pub error: Option<String>,
    pub page: Option<i64>,
}

// ─── Produse (Admin) ───────────────────────────────────────────

pub async fn admin_products_page(
    State(s): State<AdminState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<AdminQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let _user = match verify_or_redirect(&headers, &q, &*s.auth, &bp, "/admin").await {
        Ok(u) => u,
        Err(html) => return Ok(html),
    };

    let page = QueryValidator::page(q.page, 1);
    let (products, total) = s.products.get_products(None, page, ADMIN_PER_PAGE)
        .await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let total_pages = (total as f64 / ADMIN_PER_PAGE as f64).ceil() as i64;

    let products_json: Vec<serde_json::Value> = products.iter().map(|p| {
        let price_lei = p.price_new.map(|v| format!("{:.2}", v as f64 / 100.0));
        serde_json::json!({
            "id": p.id, "brand": p.brand, "name": p.name, "slug": p.slug,
            "price_new": p.price_new, "price_lei": price_lei,
            "stock_count": p.stock_count,
        })
    }).collect();

    let mut data = serde_json::json!({
        "title": "Admin — Produse",
        "products": products_json,
        "total": total,
        "page": page,
        "total_pages": total_pages,
    });
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_or_err_json(&s.renderer, "admin/admin_products.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn admin_product_new_page(
    State(s): State<AdminState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<AdminQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let _user = match verify_or_redirect(&headers, &q, &*s.auth, &bp, "/admin/product/new").await {
        Ok(u) => u,
        Err(html) => return Ok(html),
    };
    let mut data = serde_json::json!({
        "title": "Adaugă produs — Admin",
        "product": serde_json::Value::Null,
    });
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_or_err_json(&s.renderer, "admin/admin_product_form.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn admin_product_create(
    State(s): State<AdminState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<AdminQuery>,
    body: String,
) -> Response {
    let _user = match verify_or_redirect(&headers, &q, &*s.auth, &bp, "/admin/product/new").await {
        Ok(u) => u,
        Err(_) => return render_admin_redirect(&bp, "/admin/product/new").into_response(),
    };

    // 🏭 InputFactory: validează toate cîmpurile produsului
    let (brand, name, slug_s, price_new, stock_count) = match parse_any_into(&body, |fields| {
        let brand = InputFactory::parse_brand(get_field(fields, "brand")?)?;
        let name = InputFactory::parse_product_name(get_field(fields, "name")?)?;
        let slug_raw = get_field(fields, "slug").unwrap_or("");
        let slug = if slug_raw.is_empty() {
            // Auto-generare din nume
            InputFactory::parse_slug(&name.as_str().to_lowercase().replace(' ', "-"))?
        } else {
            InputFactory::parse_slug(slug_raw)?
        };
        let price_new = get_field(fields, "price_new").ok()
            .and_then(|s| s.parse::<i32>().ok());
        let stock_count = get_field(fields, "stock_count").ok()
            .and_then(|s| s.parse::<i32>().ok());
        Ok::<(String, String, String, Option<i32>, Option<i32>), InputError>((
            brand.to_string(),
            name.to_string(),
            slug.as_str().to_string(),
            price_new,
            stock_count,
        ))
    }) {
        Ok(v) => v,
        Err(e) => {
            debug_warn!(target: "admin::product", "create product: InputFactory error: {}", e);
            return error_redirect(&headers, &bp, &e.to_string());
        },
    };

    let create_req = rust_marketplace_products::CreateProductRequest {
        brand, name, slug: slug_s,
        category_id: 1, release_year: None,
        specs: Some(serde_json::json!({})),
        price_new, affiliate_url: None, image_url: None,
        stock_count,
    };

    match s.products.create_product(create_req).await {
        Ok(_) => redirect_to_admin(&headers, &bp),
        Err(e) => {
            tracing::error!(target: "admin::product", "create product eșuat: {}", e);
            error_redirect(&headers, &bp, &e.to_string())
        },
    }
}

pub async fn admin_product_edit_page(
    State(s): State<AdminState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Path(slug): Path<String>,
    Query(q): Query<AdminQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let rp = format!("/admin/product/{}/edit", &slug);
    let _user = match verify_or_redirect(&headers, &q, &*s.auth, &bp, &rp).await {
        Ok(u) => u,
        Err(html) => return Ok(html),
    };

    let product = s.products.get_by_slug(&slug).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((axum::http::StatusCode::NOT_FOUND, "Produs negăsit".into()))?;

    let mut data = serde_json::json!({
        "title": "Editează produs — Admin",
        "product": {
            "brand": product.brand, "name": product.name, "slug": product.slug,
            "price_new": product.price_new,
            "price_lei": product.price_new.map(|v| format!("{:.2}", v as f64 / 100.0)),
        },
    });
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_or_err_json(&s.renderer, "admin/admin_product_form.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub async fn admin_product_update(
    State(s): State<AdminState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Path(slug): Path<String>,
    Query(q): Query<AdminQuery>,
    body: String,
) -> Response {
    let rp = format!("/admin/product/{}/edit", &slug);
    let _user = match verify_or_redirect(&headers, &q, &*s.auth, &bp, &rp).await {
        Ok(u) => u,
        Err(_) => return render_admin_redirect(&bp, &rp).into_response(),
    };

    // 🏭 InputFactory: validează cîmpurile opționale
    let (brand, name, slug_s, price_new, stock_count) = match parse_any_into(&body, |fields| {
        let brand = get_field(fields, "brand").ok()
            .map(|s| InputFactory::parse_brand(s).map(|b| b.to_string()))
            .transpose()?;
        let name = get_field(fields, "name").ok()
            .map(|s| InputFactory::parse_product_name(s).map(|n| n.to_string()))
            .transpose()?;
        let slug_s = get_field(fields, "slug").ok()
            .map(|s| InputFactory::parse_slug(s).map(|sl| sl.as_str().to_string()))
            .transpose()?;
        let price_new = get_field(fields, "price_new").ok()
            .and_then(|s| s.parse::<i32>().ok());
        let stock_count = get_field(fields, "stock_count").ok()
            .and_then(|s| s.parse::<i32>().ok());
        Ok::<(Option<String>, Option<String>, Option<String>, Option<i32>, Option<i32>), InputError>((
            brand, name, slug_s, price_new, stock_count,
        ))
    }) {
        Ok(v) => v,
        Err(e) => {
            debug_warn!(target: "admin::product", "update product: InputFactory error: {}", e);
            return error_redirect(&headers, &bp, &e.to_string());
        },
    };

    let update_req = rust_marketplace_products::UpdateProductRequest {
        brand, name, slug: slug_s,
        category_id: None, release_year: None, specs: None,
        price_new, affiliate_url: None, image_url: None,
        stock_count,
    };

    match s.products.update_product(&slug, update_req).await {
        Ok(_) => redirect_to_admin(&headers, &bp),
        Err(e) => {
            tracing::error!(target: "admin::product", "update product {} eșuat: {}", &slug, e);
            error_redirect(&headers, &bp, &e.to_string())
        },
    }
}

fn redirect_to_admin(headers: &axum::http::HeaderMap, bp: &str) -> Response {
    let referer = headers.get("referer").and_then(|v| v.to_str().ok()).unwrap_or("");
    let dest = referer.split('?').next().unwrap_or("").to_string();
    let dest = if dest.is_empty() { format!("{}/admin", bp) } else { dest };
    // 🔒 OutputFactory: validează URL-ul redirect
    let safe_dest = OutputFactory::safe_redirect_url(&dest, "/")
        .unwrap_or_else(|| format!("{}/admin", bp));
    (StatusCode::FOUND, [("Location", safe_dest)]).into_response()
}

pub async fn admin_product_delete(
    State(s): State<AdminState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Path(slug): Path<String>,
    Query(q): Query<AdminQuery>,
) -> Response {
    let rp = format!("/admin/product/{}/delete", &slug);
    let _user = match verify_or_redirect(&headers, &q, &*s.auth, &bp, &rp).await {
        Ok(u) => u,
        Err(_) => return render_admin_redirect(&bp, &rp).into_response(),
    };
    match s.products.delete_product(&slug).await {
        Ok(_) => redirect_to_admin(&headers, &bp),
        Err(e) => {
            tracing::error!(target: "admin::product", "delete product {} eșuat: {}", &slug, e);
            error_redirect(&headers, &bp, &e.to_string())
        },
    }
}

// ─── Comenzi (Admin) ──────────────────────────────────────────

pub async fn admin_orders_page(
    State(s): State<AdminState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<AdminQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let _user = match verify_or_redirect(&headers, &q, &*s.auth, &bp, "/admin/orders").await {
        Ok(u) => u,
        Err(html) => return Ok(html),
    };

    let page = QueryValidator::page(q.page, 1);
    let offset = (page - 1) * ADMIN_PER_PAGE;
    let (orders, total) = s.orders.get_all_orders(ADMIN_PER_PAGE, offset).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let total_pages = (total as f64 / ADMIN_PER_PAGE as f64).ceil() as i64;

    let orders_json: Vec<serde_json::Value> = orders.iter().map(|o| {
        serde_json::json!({
            "id": o.id.to_string(),
            "status": o.status,
            "payment_status": o.payment_status,
            "total_lei": format!("{:.2}", o.total_bani as f64 / 100.0),
            "shipping_name": o.shipping_name,
            "shipping_address": o.shipping_address,
            "shipping_phone": o.shipping_phone,
            "created_at": o.created_at.format("%d.%m.%Y %H:%M").to_string(),
        })
    }).collect();

    let mut data = serde_json::json!({
        "title": "Admin — Comenzi",
        "orders": orders_json,
        "total": total,
        "page": page,
        "total_pages": total_pages,
    });
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_or_err_json(&s.renderer, "admin/admin_orders.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

fn error_redirect(headers: &axum::http::HeaderMap, bp: &str, msg: &str) -> Response {
    let referer = headers.get("referer").and_then(|v| v.to_str().ok()).unwrap_or("");
    let base = referer.split('?').next().unwrap_or("");
    let dest = if base.is_empty() { format!("{}/admin/orders", bp) } else { base.to_string() };
    // 🔒 OutputFactory: validează URL + sanitizează mesajul
    let safe_dest = OutputFactory::safe_redirect_url(&dest, "/")
        .unwrap_or_else(|| format!("{}/admin/orders", bp));
    let safe_msg = OutputFactory::safe_error_msg(msg);
    debug_warn!(target: "admin", "error_redirect: {} -> {} (referer: {})", msg, dest, referer);
    (StatusCode::FOUND, [("Location", format!("{}?error={}", safe_dest, url_encode(&safe_msg)))]).into_response()
}

// Folosește url_encode din crate::url_encode în loc de funcția locală

pub async fn admin_order_update_status(
    State(s): State<AdminState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Path(id): Path<uuid::Uuid>,
    Query(q): Query<AdminQuery>,
    body: String,
) -> Response {
    let rp = format!("/admin/order/{}/status", &id);
    let _user = match verify_or_redirect(&headers, &q, &*s.auth, &bp, &rp).await {
        Ok(u) => u,
        Err(_) => return render_admin_redirect(&bp, &rp).into_response(),
    };

    // 🏭 InputFactory: parsează statusul
    let status = match parse_any_into(&body, |fields| {
        let status = get_field(fields, "status")?;
        if status.is_empty() {
            return Err(InputError::MissingField("status".to_string()));
        }
        Ok(status.to_string())
    }) {
        Ok(s) => s,
        Err(e) => return error_redirect(&headers, &bp, &e.to_string()),
    };

    let order = match s.orders.get_by_id(id).await {
        Ok(Some(o)) => o,
        Ok(None) => return error_redirect(&headers, &bp, "Comanda negăsită"),
        Err(e) => return error_redirect(&headers, &bp, &e.to_string()),
    };

    let needs_payment = ["confirmed", "shipped", "delivered"];
    if needs_payment.contains(&status.as_str()) {
        // 🔒 verify_not_paid returnează Ok(când e unpaid), Err(când e paid)
        // Noi vrem invers: dacă e unpaid (Ok) → eroare, dacă e paid (Err) → ok
        if LogicFactory::verify_not_paid(&order.payment_status).is_ok() {
            let status_label = match status.as_str() {
                "confirmed" => "confirmată",
                "shipped" => "expediată",
                "delivered" => "livrată",
                _ => "procesată",
            };
            let msg = format!("Comanda nu poate fi {status_label} — plata nu a fost efectuată");
            return error_redirect(&headers, &bp, &msg);
        }
    }

    if let Err(_) = LogicFactory::verify_status_transition(&order.status, &status) {
        let msg = format!("Comanda nu poate fi {} — este deja {}",
            if status == "cancelled" { "anulată" } else { "actualizată" },
            order.status);
        return error_redirect(&headers, &bp, &msg);
    }

    if status == "cancelled" && order.payment_status == "paid" {
        if let Some(ref pid) = order.payment_provider_id {
            if let Err(e) = s.payment.refund_payment(pid).await {
                tracing::warn!("Refund eșuat pentru comanda {}: {}", id, e);
            } else {
                tracing::info!("Refund reușit pentru comanda {}", id);
                let _ = s.orders.update_payment_status(id, "refunded").await;
            }
        }
    }

    if let Err(e) = s.orders.update_status(id, &status).await {
        return error_redirect(&headers, &bp, &e.to_string());
    }

    // 🔁 Redirect înapoi la /admin/orders, nu la /admin
    let dest = format!("{}/admin/orders", bp);
    (StatusCode::FOUND, [("Location", dest)]).into_response()
}

pub async fn admin_migrate_orders(
    State(s): State<AdminState>,
    headers: axum::http::HeaderMap,
    Query(q): Query<AdminQuery>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let user = verify_admin(&headers, &q, &*s.auth).await?;

    // Acces explicit la DB — singurul loc unde încălcăm regula
    // TODO: mută în OrderRepo ca metodă migrate_user_orders()
    let updated = sqlx::query("UPDATE orders SET user_id = $1 WHERE user_id IS NULL")
        .bind(user.id)
        .execute(&s.db)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "migrated": updated.rows_affected() })))
}

// ─── Admin Logs ────────────────────────────────────────────────

pub async fn admin_logs(
    State(s): State<AdminState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<AdminQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let _user = match verify_or_redirect(&headers, &q, &*s.auth, &bp, "/admin/logs").await {
        Ok(u) => u,
        Err(html) => return Ok(html),
    };

    let log = crate::get_query_log();
    let lines: Vec<String> = log.iter().rev().take(100).cloned().collect();

    let data = serde_json::json!({
        "title": "Admin — Loguri DB",
        "lines": lines,
        "total": crate::get_query_count(),
    });
    render_or_err_json(&s.renderer, "admin/admin_logs.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}
