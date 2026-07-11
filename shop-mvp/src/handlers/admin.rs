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
use tera::Context;

use crate::state::AdminState;
use crate::render::DetectBasePath;
use crate::handlers::products::render_or_err;
use crate::types::output::OutputFactory;
use crate::debug_warn;

fn parse_body<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, String> {
    serde_json::from_str::<T>(body)
        .or_else(|_| serde_urlencoded::from_str::<T>(body))
        .map_err(|e| format!("Date invalide: {e}"))
}

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
    if user.role != "admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, "Admin: acces interzis".into()));
    }
    Ok(user)
}

fn render_admin_redirect(base_path: &str, redirect_path: &str) -> Html<String> {
    let bp = OutputFactory::text_html(base_path);
    let rp = OutputFactory::text_html(redirect_path);
    Html(format!(r#"<!DOCTYPE html><html><body><script>
window.location.replace('{bp}/login?redirect={rp}');
</script></body></html>"#))
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
                let dest = format!("{}/?error={}", bp, msg.replace(' ', "%20").replace("—", "%E2%80%94"));
                Err(Html(format!(r#"<!DOCTYPE html><html><body><script>window.location.replace('{dest}');</script></body></html>"#)))
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

    let page = q.page.unwrap_or(1).max(1);
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

    let mut ctx = Context::new();
    ctx.insert("title", "Admin — Produse");
    ctx.insert("products", &products_json);
    ctx.insert("total", &total);
    ctx.insert("page", &page);
    ctx.insert("total_pages", &total_pages);
    if let Some(ref e) = q.error { ctx.insert("error", e); }
    render_or_err(&s.renderer, "admin/admin_products.html", &ctx, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
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
    let mut ctx = Context::new();
    ctx.insert("title", "Adaugă produs — Admin");
    ctx.insert("product", &serde_json::Value::Null);
    if let Some(ref e) = q.error { ctx.insert("error", e); }
    render_or_err(&s.renderer, "admin/admin_product_form.html", &ctx, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

#[derive(Deserialize)]
pub struct CreateProductForm {
    pub brand: String,
    pub name: String,
    pub slug: String,
    pub price_new: Option<i32>,
    pub stock_count: Option<i32>,
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

    let req = match parse_body::<CreateProductForm>(&body) {
        Ok(r) => r,
        Err(e) => {
            debug_warn!(target: "admin::product", "create product: parse error: {}", e);
            return error_redirect(&headers, &bp, &e);
        },
    };

    let slug_val = if req.slug.is_empty() { req.name.clone().to_lowercase().replace(" ", "-") } else { req.slug.clone() };
    let create_req = rust_marketplace_products::CreateProductRequest {
        brand: req.brand, name: req.name, slug: slug_val,
        category_id: 1, release_year: None,
        specs: Some(serde_json::json!({})),
        price_new: req.price_new, affiliate_url: None, image_url: None,
        stock_count: req.stock_count,
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

    let mut ctx = Context::new();
    ctx.insert("title", "Editează produs — Admin");
    ctx.insert("product", &serde_json::json!({
        "brand": product.brand, "name": product.name, "slug": product.slug,
        "price_new": product.price_new,
        "price_lei": product.price_new.map(|v| format!("{:.2}", v as f64 / 100.0)),
    }));
    if let Some(ref e) = q.error { ctx.insert("error", e); }
    render_or_err(&s.renderer, "admin/admin_product_form.html", &ctx, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

#[derive(Deserialize)]
pub struct UpdateProductForm {
    pub brand: Option<String>,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub price_new: Option<i32>,
    pub stock_count: Option<i32>,
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

    let req = match parse_body::<UpdateProductForm>(&body) {
        Ok(r) => r,
        Err(e) => {
            debug_warn!(target: "admin::product", "update product: parse error: {}", e);
            return error_redirect(&headers, &bp, &e);
        },
    };

    let update_req = rust_marketplace_products::UpdateProductRequest {
        brand: req.brand, name: req.name, slug: req.slug,
        category_id: None, release_year: None, specs: None,
        price_new: req.price_new, affiliate_url: None, image_url: None,
        stock_count: req.stock_count,
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
    (StatusCode::FOUND, [("Location", dest)]).into_response()
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

    let page = q.page.unwrap_or(1).max(1);
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

    let mut ctx = Context::new();
    ctx.insert("title", "Admin — Comenzi");
    ctx.insert("orders", &orders_json);
    ctx.insert("total", &total);
    ctx.insert("page", &page);
    ctx.insert("total_pages", &total_pages);
    if let Some(ref e) = q.error { ctx.insert("error", e); }
    render_or_err(&s.renderer, "admin/admin_orders.html", &ctx, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

#[derive(Deserialize)]
pub struct AdminStatusForm {
    pub status: String,
}

fn error_redirect(headers: &axum::http::HeaderMap, bp: &str, msg: &str) -> Response {
    let referer = headers.get("referer").and_then(|v| v.to_str().ok()).unwrap_or("");
    // Elimină ?error=... din referer ca să nu se acumuleze
    let base = referer.split('?').next().unwrap_or("");
    let dest = if base.is_empty() { format!("{}/admin/orders", bp) } else { base.to_string() };
    // 🔒 OutputFactory: sanitizează mesajul de eroare
    let safe_msg = OutputFactory::safe_error_msg(msg);
    debug_warn!(target: "admin", "error_redirect: {} -> {} (referer: {})", msg, dest, referer);
    (StatusCode::FOUND, [("Location", format!("{}?error={}", dest, urlencoding(&safe_msg)))]).into_response()
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20").replace("—", "%E2%80%94")
        .replace(',', "%2C").replace("ă", "%C4%83")
        .replace("â", "%C3%A2").replace("î", "%C3%AE")
        .replace("ș", "%C8%99").replace("ț", "%C8%9B")
        .replace("Ă", "%C4%82").replace("Â", "%C3%82")
        .replace("Î", "%C3%8E").replace("Ș", "%C8%98")
        .replace("Ț", "%C8%9A")
}

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

    let req = match parse_body::<AdminStatusForm>(&body) {
        Ok(r) => r,
        Err(e) => return error_redirect(&headers, &bp, &e),
    };

    let order = match s.orders.get_by_id(id).await {
        Ok(Some(o)) => o,
        Ok(None) => return error_redirect(&headers, &bp, "Comanda negăsită"),
        Err(e) => return error_redirect(&headers, &bp, &e.to_string()),
    };

    let needs_payment = ["confirmed", "shipped", "delivered"];
    if needs_payment.contains(&req.status.as_str()) {
        if order.payment_status != "paid" {
            let status_label = match req.status.as_str() {
                "confirmed" => "confirmată",
                "shipped" => "expediată",
                "delivered" => "livrată",
                _ => "procesată",
            };
            let msg = format!("Comanda nu poate fi {status_label} — plata nu a fost efectuată");
            return error_redirect(&headers, &bp, &msg);
        }
    }

    let cannot_cancel = ["shipped", "delivered"];
    if req.status == "cancelled" && cannot_cancel.contains(&order.status.as_str()) {
        let msg = format!("Comanda nu poate fi anulată — este deja {}", if order.status == "shipped" { "expediată" } else { "livrată" });
        return error_redirect(&headers, &bp, &msg);
    }

    if req.status == "cancelled" && order.payment_status == "paid" {
        if let Some(ref pid) = order.payment_provider_id {
            if let Err(e) = s.payment.refund_payment(pid).await {
                tracing::warn!("Refund eșuat pentru comanda {}: {}", id, e);
            } else {
                tracing::info!("Refund reușit pentru comanda {}", id);
                let _ = s.orders.update_payment_status(id, "refunded").await;
            }
        }
    }

    if let Err(e) = s.orders.update_status(id, &req.status).await {
        return error_redirect(&headers, &bp, &e.to_string());
    }

    redirect_to_admin(&headers, &bp)
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

    let mut ctx = Context::new();
    ctx.insert("title", "Admin — Loguri DB");
    ctx.insert("lines", &lines);
    ctx.insert("total", &crate::get_query_count());
    render_or_err(&s.renderer, "admin/admin_logs.html", &ctx, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}
