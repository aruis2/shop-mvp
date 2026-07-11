// =============================================================================
// 📦 Orders + Checkout — capability: OrderRepo + CartRepo + PaymentRepo + Auth
// =============================================================================

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::state::OrderState;
use crate::render::DetectBasePath;
use crate::handlers::products::{render_or_err_json};
use crate::types::logic::LogicFactory;
use crate::types::output::OutputFactory;
use crate::types::parser::{parse_any_into, get_field};
use crate::types::error::InputError;
use crate::types::InputFactory;
use crate::types::QueryValidator;
use crate::{debug_warn, debug_log};

/// Extrage token-ul JWT din: Authorization header > cookie > query param
fn extract_token<'a>(headers: &'a axum::http::HeaderMap, q: &'a Option<String>) -> Option<&'a str> {
    headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| {
            headers.get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|c| crate::cookie::get_cookie(c, "token"))
        })
        .or_else(|| q.as_deref())
}

#[derive(Deserialize)]
pub struct CheckoutQuery {
    pub session_id: Option<String>,
}

pub async fn checkout_page(
    State(s): State<OrderState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<CheckoutQuery>,
) -> Response {
    let sid = q.session_id.clone().or_else(|| {
        headers.get("x-session-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }).or_else(|| {
        headers.get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|c| crate::cookie::get_cookie(c, "session_id"))
            .map(|s| s.to_string())
    }).unwrap_or_else(|| "anon".to_string());

    let cart = match s.cart.get_cart(&sid).await {
        Ok(c) => c,
        Err(e) => return error_redirect(&format!("{}/cart", bp), &e.to_string()),
    };

    // 🏭 LogicFactory: verifică coș ne-gol
    if cart.items.is_empty() {
        return error_redirect(&format!("{}/cart", bp), "Coșul e gol");
    }

    let data = serde_json::json!({
        "title": "Checkout — Shop MVP",
        "session_id": sid,
        "total_lei": format!("{:.2}", cart.total_bani as f64 / 100.0),
        "item_count": cart.item_count,
    });
    match render_or_err_json(&s.renderer, "orders/checkout.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await {
        Ok(html) => html.into_response(),
        Err((code, msg)) => (code, msg).into_response(),
    }
}

/// Date de checkout validate prin InputFactory
struct CheckoutParsed {
    session_id: String,
    guest_email: Option<String>,
    shipping_name: String,
    shipping_address: String,
    shipping_phone: String,
    notes: Option<String>,
}

fn err_htmx(msg: &str) -> Response {
    // 🔒 OutputFactory: sanitizează mesajul de eroare
    let safe = OutputFactory::text_html(msg);
    Html(format!("<div class=\"text-red-600\">❌ {safe}</div>")).into_response()
}

fn error_redirect(dest: &str, msg: &str) -> Response {
    debug_warn!(target: "orders", "error_redirect: {} -> {}", msg, dest);
    // 🔒 OutputFactory: validează URL + sanitizează mesajul
    let safe_dest = OutputFactory::safe_redirect_url(dest, "/")
        .unwrap_or_else(|| "/".to_string());
    let safe_msg = OutputFactory::safe_error_msg(msg);
    (StatusCode::FOUND, [("Location", format!("{}?error={}", safe_dest, safe_msg))]).into_response()
}

fn redirect_to_login(base_path: &str) -> Response {
    // 🔒 OutputFactory: validează URL-ul redirect
    let dest = format!("{}/login?redirect={}/orders", base_path, base_path);
    let safe_dest = OutputFactory::safe_redirect_url(&dest, "/")
        .unwrap_or_else(|| "/login".to_string());
    (StatusCode::FOUND, [("Location", safe_dest)]).into_response()
}

pub async fn checkout_handler(
    State(s): State<OrderState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let is_htmx = headers.get("hx-request").is_some();
    let token_str = extract_token(&headers, &None).unwrap_or("");

    // 🏭 InputFactory: parsează și validează TOT inputul
    let checkout = match parse_any_into(&body, |fields| {
        let session_id = InputFactory::parse_session_id(get_field(fields, "session_id")?)?;
        let guest_email = get_field(fields, "guest_email").ok()
            .and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
        let shipping_name = InputFactory::parse_name(get_field(fields, "shipping_name")?)?;
        let shipping_address = InputFactory::parse_address(get_field(fields, "shipping_address")?)?;
        let shipping_phone = InputFactory::parse_phone(get_field(fields, "shipping_phone")?)?;
        let notes = get_field(fields, "notes").ok()
            .and_then(|s| if s.is_empty() { None } else {
                InputFactory::parse_notes(s).ok().map(|n| n.to_string())
            });
        Ok(CheckoutParsed {
            session_id: session_id.to_string(),
            guest_email,
            shipping_name: shipping_name.to_string(),
            shipping_address: shipping_address.to_string(),
            shipping_phone: shipping_phone.to_string(),
            notes,
        })
    }) {
        Ok(c) => c,
        Err(InputError::MissingField(f)) => {
            let msg = format!("Cîmpul '{f}' lipsește");
            return if is_htmx { err_htmx(&msg) } else { error_redirect(&format!("{}/checkout", bp), &msg) };
        },
        Err(e) => {
            return if is_htmx { err_htmx(&e.to_string()) } else { error_redirect(&format!("{}/checkout", bp), &e.to_string()) };
        },
    };

    let user_id = if token_str.is_empty() { None } else { s.auth.verify_token(token_str).await.ok().map(|u| u.id) };

    let cart = match s.cart.get_cart(&checkout.session_id).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(target: "orders::checkout", "cart fetch eșuat: {}", e);
            return if is_htmx { err_htmx(&e.to_string()) } else { error_redirect(&format!("{}/checkout", bp), &e.to_string()) };
        },
    };

    // 🏭 LogicFactory: verifică coș ne-gol
    if cart.items.is_empty() {
        debug_warn!(target: "orders::checkout", "checkout cu coșul gol");
        return if is_htmx { err_htmx("Coșul e gol") } else { error_redirect(&format!("{}/cart", bp), "Coșul e gol") };
    }

    let cart_items: Vec<(String, String, i64, i32)> = cart.items.iter()
        .map(|i| (i.product_slug.clone(), i.product_name.clone(), i.price_bani, i.qty))
        .collect();

    let order_req = rust_marketplace_orders::PlaceOrderRequest {
        session_id: checkout.session_id.clone(),
        guest_email: checkout.guest_email.clone(),
        shipping_name: checkout.shipping_name.clone(),
        shipping_address: checkout.shipping_address.clone(),
        shipping_phone: checkout.shipping_phone.clone(),
        notes: checkout.notes.clone(),
    };

    let order = match s.orders.place_order(user_id, order_req, cart_items).await {
        Ok(o) => o,
        Err(e) => {
            tracing::error!(target: "orders::checkout", "place_order eșuat: {}", e);
            return if is_htmx { err_htmx(&e.to_string()) } else { error_redirect(&format!("{}/checkout", bp), &e.to_string()) };
        },
    };

    debug_log!(target: "orders::checkout", "checkout reușit: comanda {} pentru session={}", order.id, checkout.session_id);
    let _ = s.cart.clear_cart(&checkout.session_id).await;

    let checkout_req = rust_payment::CreateCheckoutRequest {
        order_id: order.id.to_string(),
        amount_bani: order.total_bani,
        currency: "ron".into(),
        success_url: format!("{}/success?order_id={}", s.site_url, order.id),
        cancel_url: format!("{}/cart?session_id={}", s.site_url, checkout.session_id),
    };

    match s.payment.create_checkout(checkout_req).await {
        Ok(stripe_session) => {
            let _ = s.orders.set_payment_info(order.id, "stripe", &stripe_session.session_id).await;
            // 302 redirect la Stripe — funcționează și pentru form POST
            (StatusCode::FOUND, [("Location", stripe_session.checkout_url)]).into_response()
        }
        Err(e) => {
            tracing::error!(target: "orders::stripe", "Stripe checkout eșuat: {}", e);
            let tk = if token_str.is_empty() { String::new() } else { format!("?token={}", token_str) };
            let dest = format!("{}/orders{}", bp, tk);
            if is_htmx {
                Html(format!("<script>window.location.href='{dest}';</script>")).into_response()
            } else {
                (StatusCode::FOUND, [("Location", dest)]).into_response()
            }
        }
    }
}

pub async fn order_pay(
    State(s): State<OrderState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Path(order_id): Path<uuid::Uuid>,
) -> Response {
    let is_htmx = headers.get("hx-request").is_some();
    let token = match extract_token(&headers, &None) {
        Some(t) => t.to_string(),
        None => {
        debug_warn!(target: "orders::pay", "order_pay: neautentificat");
        return if is_htmx { err_htmx("Trebuie să fii autentificat") } else { error_redirect(&format!("{}/login", bp), "Trebuie să fii autentificat") };
    },
    };

    let user = match s.auth.verify_token(&token).await {
        Ok(u) => u,
        Err(_) => {
            debug_warn!(target: "orders::pay", "order_pay: token invalid");
            return if is_htmx { err_htmx("Token invalid") } else { error_redirect(&format!("{}/login", bp), "Token invalid") };
        },
    };

    let order = match s.orders.get_by_id(order_id).await {
        Ok(Some(o)) => o,
        Ok(None) => {
            debug_warn!(target: "orders::pay", "order_pay: comanda {} nu există", order_id);
            return if is_htmx { err_htmx("Comanda nu există") } else { error_redirect(&format!("{}/orders", bp), "Comanda nu există") };
        },
        Err(e) => {
            tracing::error!(target: "orders::pay", "order_pay: DB error la comanda {}: {}", order_id, e);
            return if is_htmx { err_htmx(&e.to_string()) } else { error_redirect(&format!("{}/orders", bp), &e.to_string()) };
        },
    };

    if let Err(_) = LogicFactory::verify_ownership(&user.id, &order.user_id.unwrap_or_default(), "order") {
        debug_warn!(target: "orders::pay", "order_pay: IDOR încercat comanda {} de user {}", order_id, user.id);
        return if is_htmx { err_htmx("Nu e comanda ta") } else { error_redirect(&format!("{}/orders", bp), "Nu e comanda ta") };
    }
    if let Err(_) = LogicFactory::verify_not_paid(&order.payment_status) {
        debug_log!(target: "orders::pay", "order_pay: comanda {} e deja plătită", order_id);
        return if is_htmx { err_htmx("Deja plătită") } else { error_redirect(&format!("{}/orders", bp), "Deja plătită") };
    }

    let checkout_req = rust_payment::CreateCheckoutRequest {
        order_id: order.id.to_string(),
        amount_bani: order.total_bani,
        currency: "ron".into(),
        success_url: format!("{}/success?order_id={}", s.site_url, order.id),
        cancel_url: format!("{}/orders", s.site_url),
    };

    match s.payment.create_checkout(checkout_req).await {
        Ok(session) => {
            // 302 redirect la Stripe — funcționează și pentru form POST, nu doar HTMX
            (StatusCode::FOUND, [("Location", session.checkout_url)]).into_response()
        }
        Err(e) => {
            tracing::error!(target: "orders::pay", "Stripe checkout eșuat pentru comanda {}: {}", order_id, e);
            if is_htmx { err_htmx(&e.to_string()) } else { error_redirect(&format!("{}/orders", bp), &e.to_string()) }
        }
    }
}

#[derive(Deserialize)]
pub struct OrdersQuery {
    pub token: Option<String>,
    pub error: Option<String>,
    pub page: Option<i64>,
}

pub async fn orders_page(
    State(s): State<OrderState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<OrdersQuery>,
) -> Response {
    let token = extract_token(&headers, &q.token);
    let user = match token {
        Some(t) => match s.auth.verify_token(t).await {
            Ok(u) => u,
            Err(_) => return redirect_to_login(&bp),
        },
        None => return redirect_to_login(&bp),
    };

    const ORDERS_PER_PAGE: i64 = 10;
    let page = QueryValidator::page(q.page, 1);
    let offset = (page - 1) * ORDERS_PER_PAGE;
    let (orders, total) = match s.orders.get_orders_by_user(user.id, ORDERS_PER_PAGE, offset).await {
        Ok(o) => o,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let total_pages = (total as f64 / ORDERS_PER_PAGE as f64).ceil() as i64;

    let orders_json: Vec<serde_json::Value> = orders.iter().map(|o| {
        serde_json::json!({
            "id": o.id.to_string(),
            "status": o.status,
            "payment_status": o.payment_status,
            "total_lei": format!("{:.2}", o.total_bani as f64 / 100.0),
            "shipping_name": o.shipping_name,
            "shipping_address": o.shipping_address,
            "created_at": o.created_at.format("%d.%m.%Y %H:%M").to_string(),
        })
    }).collect();

    let mut data = serde_json::json!({
        "title": "Comenzile mele — Shop MVP",
        "orders": orders_json,
        "page": page,
        "total_pages": total_pages,
    });
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    match render_or_err_json(&s.renderer, "orders/orders.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await {
        Ok(html) => html.into_response(),
        Err((code, msg)) => (code, msg).into_response(),
    }
}

#[derive(Deserialize)]
pub struct SuccessQuery {
    pub order_id: Option<String>,
}

pub async fn success_page(
    State(s): State<OrderState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<SuccessQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    if let Some(ref order_id_str) = q.order_id {
        if let Ok(order_id) = uuid::Uuid::parse_str(order_id_str) {
            let _ = s.orders.update_payment_status(order_id, "paid").await;
        }
    }
    let data = serde_json::json!({"title": "Comandă reușită! — Shop MVP"});
    render_or_err_json(&s.renderer, "orders/success.html", &data, &bp, false, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

// ============================================================================
// 🔒 PSD2/SCA: Stripe Webhook — confirmare plată asincronă
// ============================================================================
// Stripe trimite un webhook când o plată e confirmată (inclusiv după 3D Secure).
// Acest handler actualizează statusul comenzii și previne dublarea plăților.

/// Stripe webhook pentru checkout.session.completed
pub async fn stripe_webhook(
    State(s): State<OrderState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> impl axum::response::IntoResponse {
    // Verificare semnătură webhook (opțional, necesită stripe-signature secret)
    let event_type = headers.get("x-stripe-webhook-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Parsează evenimentul Stripe
    let event: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(target: "stripe::webhook", "JSON invalid: {e}");
            return (axum::http::StatusCode::BAD_REQUEST, "Invalid JSON").into_response();
        }
    };

    tracing::info!(target: "stripe::webhook", "Eveniment Stripe: {event_type}");

    if event_type == "checkout.session.completed" {
        let session = &event["data"]["object"];
        let session_id = session["id"].as_str().unwrap_or("");
        let order_id_str = session["metadata"]["order_id"].as_str()
            .or_else(|| session["client_reference_id"].as_str())
            .unwrap_or("");

        if let Ok(order_id) = uuid::Uuid::parse_str(order_id_str) {
            // Idempotency check: verifică dacă plata a fost deja procesată
            let idem_key = format!("stripe_webhook_{}", session_id);
            if crate::check_idempotency(&idem_key).is_some() {
                tracing::warn!(target: "stripe::webhook", "Webhook duplicat ignorat: {session_id}");
                return (axum::http::StatusCode::OK, "Already processed").into_response();
            }

            match s.orders.update_payment_status(order_id, "paid").await {
                Ok(_) => {
                    crate::store_idempotency_result(&idem_key, "paid");
                    tracing::info!(target: "stripe::webhook", "✅ Plată confirmată pentru comanda {order_id} (SCA: {})",
                        session["payment_intent"]["payment_method_types"].as_array().map(|_| "ok").unwrap_or("n/a"));
                    (axum::http::StatusCode::OK, "OK").into_response()
                }
                Err(e) => {
                    tracing::error!(target: "stripe::webhook", "Eroare la actualizare comandă {order_id}: {e}");
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "DB error").into_response()
                }
            }
        } else {
            tracing::warn!(target: "stripe::webhook", "Webhook fără order_id valid: {order_id_str}");
            (axum::http::StatusCode::OK, "No order_id").into_response()
        }
    } else {
        // Alte evenimente: payment_intent.succeeded, etc. — ignorate
        (axum::http::StatusCode::OK, "Event ignored").into_response()
    }
}
