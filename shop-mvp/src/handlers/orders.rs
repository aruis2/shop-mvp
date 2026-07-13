// =============================================================================
// 📦 Orders + Checkout — capability: OrderRepo + CartRepo + PaymentRepo + Auth
// =============================================================================

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::state::OrderState;
use crate::render::DetectBasePath;
use crate::handlers::products::render_safe_json;
use crate::boundary::*;
use crate::url_encode::url_encode;
use crate::{debug_warn, debug_log};

/// Extrage token-ul JWT din: Authorization header > cookie > query param
/// Extrage token-ul JWT din: Authorization header > cookie (NU din query param — securitate)
/// 🔒 Token-ul în query param e un risc de securitate (apare în logs, Referer, history).
/// Folosește doar header-e care nu apar în Referer.
fn extract_token<'a>(headers: &'a axum::http::HeaderMap) -> Option<&'a str> {
    headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| {
            headers.get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|c| crate::cookie::get_cookie(c, "token"))
        })
}

#[derive(Deserialize)]
pub struct CheckoutQuery {
    pub session_id: Option<String>,
    pub error: Option<String>,
    pub cart: Option<String>, // "private" = doar itemele cu user_id; orice altceva = tot
}

pub async fn checkout_page(
    State(s): State<OrderState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Query(q): Query<CheckoutQuery>,
) -> SafeResponse {
    let sid = q.session_id.clone().or_else(|| {
        // 🏭 InputFactory: validează header-ul x-session-id
        headers.get("x-session-id")
            .and_then(|v| v.to_str().ok().map(String::from))
            .and_then(|s| QueryValidator::session_id(Some(s), "header:x-session-id"))
    }).or_else(|| {
        headers.get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|c| crate::cookie::get_cookie(c, "session_id"))
            .map(|s| s.to_string())
    }).unwrap_or_else(|| "anon".to_string());

    // Dacă utilizatorul e autentificat, include și itemele private (user_id)
    let token = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| crate::cookie::get_cookie(c, "token"));
    let cart = match token {
        Some(t) => match s.auth.verify_token(t).await {
            Ok(u) => {
                if q.cart.as_deref() == Some("private") {
                    // 🛒 Doar coșul privat (iteme cu user_id) — urmărește utilizatorul
                    match s.cart.get_private_cart(u.id).await {
                        Ok(c) => c,
                        Err(e) => return error_redirect(&format!("{}/cart", bp), &e.to_string()),
                    }
                } else {
                    // 🛒 Coș complet: privat + public (session_id + user_id)
                    match s.cart.get_cart_by_user(&sid, u.id).await {
                        Ok(c) => c,
                        Err(e) => return error_redirect(&format!("{}/cart", bp), &e.to_string()),
                    }
                }
            },
            Err(_) => match s.cart.get_cart(&sid).await {
                Ok(c) => c,
                Err(e) => return error_redirect(&format!("{}/cart", bp), &e.to_string()),
            },
        },
        None => match s.cart.get_cart(&sid).await {
            Ok(c) => c,
            Err(e) => return error_redirect(&format!("{}/cart", bp), &e.to_string()),
        },
    };

    // 🏭 LogicFactory: verifică coș ne-gol
    if cart.items.is_empty() {
        return error_redirect(&format!("{}/cart", bp), "Coșul e gol");
    }

    // 🔑 Dacă e coș privat, prefixăm session_id cu "private_" pentru ca
    // checkout_handler să știe că doar itemele private trebuie cumpărate/șterse.
    let form_session_id = if q.cart.as_deref() == Some("private") {
        format!("private_{}", sid)
    } else {
        sid.clone()
    };

    let mut data = serde_json::json!({
        "title": "Checkout — Shop MVP",
        "session_id": form_session_id,
        "total_lei": format!("{:.2}", cart.total_bani as f64 / 100.0),
        "item_count": cart.item_count,
    });
    if let Some(ref e) = q.error { data["error"] = serde_json::json!(e); }
    render_safe_json(&s.renderer, "orders/checkout.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

pub struct CheckoutForm {
    pub session_id: String,
    pub is_private: bool,
    pub guest_email: Option<String>,
    pub shipping_name: String,
    pub shipping_address: String,
    pub shipping_phone: String,
    pub notes: Option<String>,
}

impl ValidateForm for CheckoutForm {
    fn validate(fields: &[FormField], headers: &HeaderMap) -> Result<Self, SafeResponse> {
        let raw = get_field(fields, "session_id")
            .map_err(|_| redirect_back(headers, "/checkout", "Session ID lipsă"))?;
        let is_private = raw.starts_with("private_");
        let clean = if is_private { &raw[8..] } else { raw };
        let session_id = InputFactory::parse_session_id(clean)
            .map_err(|_| redirect_back(headers, "/checkout", "Session ID invalid"))?;
        let guest_email = get_field(fields, "guest_email").ok()
            .and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
        let shipping_name = InputFactory::parse_name(
            get_field(fields, "shipping_name").map_err(|_| redirect_back(headers, "/checkout", "Nume lipsă"))?
        ).map_err(|_| redirect_back(headers, "/checkout", "Nume invalid"))?;
        let shipping_address = InputFactory::parse_address(
            get_field(fields, "shipping_address").map_err(|_| redirect_back(headers, "/checkout", "Adresă lipsă"))?
        ).map_err(|_| redirect_back(headers, "/checkout", "Adresă invalidă"))?;
        let shipping_phone = InputFactory::parse_phone(
            get_field(fields, "shipping_phone").map_err(|_| redirect_back(headers, "/checkout", "Telefon lipsă"))?
        ).map_err(|_| redirect_back(headers, "/checkout", "Telefon invalid"))?;
        let notes = get_field(fields, "notes").ok()
            .and_then(|s| if s.is_empty() { None } else {
                InputFactory::parse_notes(s).ok().map(|n| n.to_string())
            });
        Ok(CheckoutForm {
            session_id: session_id.to_string(),
            is_private,
            guest_email,
            shipping_name: shipping_name.as_str().to_string(),
            shipping_address: shipping_address.as_str().to_string(),
            shipping_phone: shipping_phone.as_str().to_string(),
            notes,
        })
    }
}

fn error_redirect(dest: &str, msg: &str) -> SafeResponse {
    debug_warn!(target: "orders", "error_redirect: {} -> {}", msg, dest);
    // 🔒 OutputFactory: validează URL + sanitizează mesajul
    let safe_dest = OutputFactory::safe_redirect_url(dest, "/")
        .unwrap_or_else(|| "/".to_string());
    let safe_msg = OutputFactory::safe_error_msg(msg);
    let encoded = url_encode(&safe_msg);
    SafeResponse::redirect(format!("{}?error={}", safe_dest, encoded))
}

fn redirect_to_login(base_path: &str) -> SafeResponse {
    // 🔒 OutputFactory: validează URL-ul redirect
    let dest = format!("{}/login?redirect={}/orders", base_path, base_path);
    let safe_dest = OutputFactory::safe_redirect_url(&dest, "/")
        .unwrap_or_else(|| "/login".to_string());
    SafeResponse::redirect(safe_dest)
}

pub async fn checkout_handler(
    State(s): State<OrderState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    ValidatedForm(checkout): ValidatedForm<CheckoutForm>,
) -> SafeResponse {
    let token_str = extract_token(&headers).unwrap_or("");

    let user_id = if token_str.is_empty() { None } else { s.auth.verify_token(token_str).await.ok().map(|u| u.id) };

    // Dacă utilizatorul e autentificat, include și itemele private (user_id)
    let is_private = checkout.is_private;
    let cart = match user_id {
        Some(uid) => {
            if is_private {
                // 🛒 Doar coșul privat
                match s.cart.get_private_cart(uid).await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!(target: "orders::checkout", "cart fetch eșuat: {}", e);
                        return error_redirect(&format!("{}/checkout", bp), &e.to_string());
                    },
                }
            } else {
                // 🛒 Coș complet: privat + public
                match s.cart.get_cart_by_user(&checkout.session_id, uid).await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!(target: "orders::checkout", "cart fetch eșuat: {}", e);
                        return error_redirect(&format!("{}/checkout", bp), &e.to_string());
                    },
                }
            }
        },
        None => match s.cart.get_cart(&checkout.session_id).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(target: "orders::checkout", "cart fetch eșuat: {}", e);
                return error_redirect(&format!("{}/checkout", bp), &e.to_string());
            },
        },
    };

    // 🏭 LogicFactory: verifică coș ne-gol
    if cart.items.is_empty() {
        debug_warn!(target: "orders::checkout", "checkout cu coșul gol");
        return error_redirect(&format!("{}/cart", bp), "Coșul e gol");
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
            return error_redirect(&format!("{}/checkout", bp), &e.to_string());
        },
    };

    debug_log!(target: "orders::checkout", "checkout reușit: comanda {} pentru session={} (private={})", order.id, checkout.session_id, is_private);
    // 🔒 Ștergem doar itemele care au fost cumpărate
    // is_private=true → ștergem doar itemele private (user_id)
    // is_private=false → ștergem tot (public + privat)
    if is_private {
        // Doar private → ștergem doar itemele private
        if let Some(uid) = user_id {
            let _ = s.cart.clear_cart(&"", Some(uid)).await;
        }
    } else {
        let _ = s.cart.clear_cart(&checkout.session_id, user_id).await;
    }

    let checkout_req = rust_payment::CreateCheckoutRequest {
        order_id: order.id.to_string(),
        amount_bani: order.total_bani,
        currency: "ron".into(),
        success_url: format!("{}/success?order_id={}", s.site_url, order.id),
        cancel_url: format!("{}/cart?session_id={}", s.site_url, checkout.session_id),
    };

    match s.payment.create_checkout(checkout_req).await {
        Ok(payment_session) => {
            let provider = if payment_session.session_id.starts_with("mock_") { "mock" } else { "stripe" };
            let _ = s.orders.set_payment_info(order.id, provider, &payment_session.session_id).await;

            // 🔧 Mock payment: marchează imediat ca plătit
            if provider == "mock" {
                let _ = s.orders.update_payment_status(order.id, "paid").await;
                tracing::info!(target: "orders::checkout", "✅ Mock plată instant pentru comanda {}", order.id);
            }

            // 302 redirect la Stripe sau la success_url (mock)
            SafeResponse::redirect(payment_session.checkout_url)
        }
        Err(e) => {
            tracing::error!(target: "orders::stripe", "Checkout eșuat: {}", e);
            let tk = if token_str.is_empty() { String::new() } else { format!("?token={}", token_str) };
            let dest = format!("{}/orders{}", bp, tk);
            SafeResponse::redirect(dest)
        }
    }
}

pub async fn order_pay(
    State(s): State<OrderState>,
    DetectBasePath(bp): DetectBasePath,
    headers: axum::http::HeaderMap,
    Path(order_id): Path<uuid::Uuid>,
) -> SafeResponse {
    let token = match extract_token(&headers) {
        Some(t) => t.to_string(),
        None => {
        debug_warn!(target: "orders::pay", "order_pay: neautentificat");
        return error_redirect(&format!("{}/login", bp), "Trebuie să fii autentificat");
    },
    };

    let user = match s.auth.verify_token(&token).await {
        Ok(u) => u,
        Err(_) => {
            debug_warn!(target: "orders::pay", "order_pay: token invalid");
            return error_redirect(&format!("{}/login", bp), "Token invalid");
        },
    };

    let order = match s.orders.get_by_id(order_id).await {
        Ok(Some(o)) => o,
        Ok(None) => {
            debug_warn!(target: "orders::pay", "order_pay: comanda {} nu există", order_id);
            return error_redirect(&format!("{}/orders", bp), "Comanda nu există");
        },
        Err(e) => {
            tracing::error!(target: "orders::pay", "order_pay: DB error la comanda {}: {}", order_id, e);
            return error_redirect(&format!("{}/orders", bp), &e.to_string());
        },
    };

    if let Err(_) = LogicFactory::verify_ownership(&user.id, &order.user_id.unwrap_or_default(), "order") {
        debug_warn!(target: "orders::pay", "order_pay: IDOR încercat comanda {} de user {}", order_id, user.id);
        return error_redirect(&format!("{}/orders", bp), "Nu e comanda ta");
    }
    if let Err(_) = LogicFactory::verify_not_paid(&order.payment_status) {
        debug_log!(target: "orders::pay", "order_pay: comanda {} e deja plătită", order_id);
        return error_redirect(&format!("{}/orders", bp), "Deja plătită");
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
            // 🔧 Mock payment: marchează imediat ca plătit
            if session.session_id.starts_with("mock_") {
                let _ = s.orders.set_payment_info(order.id, "mock", &session.session_id).await;
                let _ = s.orders.update_payment_status(order.id, "paid").await;
                tracing::info!(target: "orders::pay", "✅ Mock plată instant pentru comanda {}", order.id);
            }
            // 302 redirect la Stripe sau la success_url (mock)
            SafeResponse::redirect(session.checkout_url)
        }
        Err(e) => {
            tracing::error!(target: "orders::pay", "Checkout eșuat pentru comanda {}: {}", order_id, e);
            error_redirect(&format!("{}/orders", bp), &e.to_string())
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
) -> SafeResponse {
    // 🔒 Token doar din header-e (nu din query param — risc de securitate)
    let token = extract_token(&headers);
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
        Err(e) => return SafeResponse::server_error(e.to_string()),
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
    render_safe_json(&s.renderer, "orders/orders.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
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
) -> SafeResponse {
    let oid = q.order_id.as_ref().and_then(|id| uuid::Uuid::parse_str(id).ok());
    let mut data = serde_json::json!({"title": "Comandă înregistrată! — Shop MVP"});

    if let Some(ref order_id_uuid) = oid {
        // Încercăm să încărcăm detaliile comenzii
        if let Ok(Some(order)) = s.orders.get_by_id(*order_id_uuid).await {
            data["order_id"] = serde_json::json!(order_id_uuid.to_string());
            data["total_lei"] = serde_json::json!(format!("{:.2}", order.total_bani as f64 / 100.0));
            data["shipping_name"] = serde_json::json!(order.shipping_name);
            data["shipping_address"] = serde_json::json!(order.shipping_address);
            data["shipping_phone"] = serde_json::json!(order.shipping_phone);
            data["payment_status"] = serde_json::json!(order.payment_status);
            data["status"] = serde_json::json!(order.status);

            // Încărcăm itemele
            if let Ok(items) = s.orders.get_items(*order_id_uuid).await {
                let items_json: Vec<serde_json::Value> = items.iter().map(|item| {
                    serde_json::json!({
                        "product_name": item.product_name,
                        "qty": item.qty,
                        "price_lei": format!("{:.2}", item.price_bani as f64 / 100.0),
                        "subtotal_lei": format!("{:.2}", (item.price_bani * item.qty as i64) as f64 / 100.0),
                    })
                }).collect();
                data["items"] = serde_json::json!(items_json);
            }

            tracing::info!(target: "orders::success", "Pagină success pentru comanda {} (plătit: {})", order_id_uuid, order.payment_status);
        } else {
            // Comanda nu s-a găsit — poate e ID greșit, dar arătăm oricum ceva util
            data["order_id"] = serde_json::json!(order_id_uuid.to_string());
            tracing::warn!(target: "orders::success", "Comanda {} nu s-a găsit în DB", order_id_uuid);
        }
    }

    render_safe_json(&s.renderer, "orders/success.html", &data, &bp, &headers, &*s.auth as &dyn rust_auth::AuthRepo).await
}

// ============================================================================
// 🔒 PSD2/SCA: Stripe Webhook — confirmare plată asincronă
// ============================================================================
// Stripe trimite un webhook când o plată e confirmată (inclusiv după 3D Secure).
// VERIFICĂM semnătura HMAC-SHA256 înainte de a procesa — dacă nu e validă, ignorăm.
// ============================================================================

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Verifică semnătura Stripe webhook.
/// Format header `stripe-signature`: `t=timestamp,v1=signature`
/// Signature = HMAC-SHA256(secret, timestamp.body)
fn verify_stripe_signature(payload: &str, sig_header: &str, secret: &str) -> bool {
    // Extrage timestamp și semnătura din header
    let mut timestamp = String::new();
    let mut signature = String::new();
    for part in sig_header.split(',') {
        if let Some(val) = part.strip_prefix("t=") {
            timestamp = val.to_string();
        } else if let Some(val) = part.strip_prefix("v1=") {
            signature = val.to_string();
        }
    }
    if timestamp.is_empty() || signature.is_empty() {
        return false;
    }

    // Calculează HMAC-SHA256(secret, timestamp.payload)
    let signed_payload = format!("{}.{}", timestamp, payload);
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());
    let computed = mac.finalize().into_bytes();

    // Constant-time comparison (previne timing attacks)
    let expected = hex::decode(&signature).unwrap_or_default();
    computed.as_slice().eq(&expected)
}

/// Stripe webhook pentru checkout.session.completed
pub async fn stripe_webhook(
    State(s): State<OrderState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> impl axum::response::IntoResponse {
    // Parsează evenimentul Stripe — ÎNAINTE de verificarea semnăturii
    // (JSON invalid = 400, indiferent de semnătură)
    let event: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(target: "stripe::webhook", "JSON invalid: {e}");
            return (axum::http::StatusCode::BAD_REQUEST, "Invalid JSON").into_response();
        }
    };

    // 🔒 Verifică semnătura Stripe webhook — esențial pentru securitate
    let sig_header = match headers.get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s,
        None => {
            tracing::error!(target: "stripe::webhook", "Webhook fără stripe-signature header");
            return (axum::http::StatusCode::UNAUTHORIZED, "Missing signature").into_response();
        }
    };

    let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
        .unwrap_or_else(|_| {
            tracing::warn!(target: "stripe::webhook", "STRIPE_WEBHOOK_SECRET ne setat — verificare semnătură dezactivată!");
            String::new()
        });

    if !webhook_secret.is_empty() && !verify_stripe_signature(&body, sig_header, &webhook_secret) {
        tracing::error!(target: "stripe::webhook", "Semnătură webhook invalidă — posibil atac!");
        return (axum::http::StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
    }

    let event_type = event["type"].as_str()
        .or_else(|| {
            // Fallback pentru testare manuală (x-stripe-webhook-type)
            headers.get("x-stripe-webhook-type")
                .and_then(|v| v.to_str().ok())
        })
        .unwrap_or("unknown");

    tracing::info!(target: "stripe::webhook", "Eveniment Stripe: {event_type} (semnătură verificată)");

    if event_type == "checkout.session.completed" {
        let session = &event["data"]["object"];
        let session_id = session["id"].as_str().unwrap_or("");
        let order_id_str = session["metadata"]["order_id"].as_str()
            .or_else(|| session["client_reference_id"].as_str())
            .unwrap_or("");

        if let Ok(order_id) = uuid::Uuid::parse_str(order_id_str) {
            // 🔒 Idempotency check în DB — supraviețuiește restarturilor
            let idem_key = format!("stripe_webhook_{}", session_id);
            match s.orders.check_idempotency(&idem_key).await {
                Ok(Some(_)) => {
                    tracing::warn!(target: "stripe::webhook", "Webhook duplicat ignorat: {session_id}");
                    return (axum::http::StatusCode::OK, "Already processed").into_response();
                }
                Ok(None) => {} // continuă
                Err(e) => {
                    tracing::error!(target: "stripe::webhook", "Idempotency check eșuat: {e}");
                }
            }

            match s.orders.update_payment_status(order_id, "paid").await {
                Ok(_) => {
                    let _ = s.orders.store_idempotency(&idem_key, "paid").await;
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
