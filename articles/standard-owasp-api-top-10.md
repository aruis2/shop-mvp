---
title: "OWASP API Top 10 — Securitate API REST"
description: "Implementarea OWASP API Security Top 10 în shop-mvp"
date: 2026-07-11
---

# OWASP API Security Top 10

## Cuprins

1. [Introducere](#introducere)
2. [API1: Broken Object Level Authorization](#api1-broken-object-level-authorization)
3. [API2: Broken Authentication](#api2-broken-authentication)
4. [API3: Broken Object Property Level Authorization](#api3-broken-object-property-level-authorization)
5. [API4: Unrestricted Resource Consumption](#api4-unrestricted-resource-consumption)
6. [API5: Broken Function Level Authorization](#api5-broken-function-level-authorization)
7. [API6: Unrestricted Access to Sensitive Business Flows](#api6-unrestricted-access-to-sensitive-business-flows)
8. [API7: Server Side Request Forgery](#api7-server-side-request-forgery)
9. [API8: Security Misconfiguration](#api8-security-misconfiguration)
10. [API9: Improper Inventory Management](#api9-improper-inventory-management)
11. [API10: Unsafe Consumption of APIs](#api10-unsafe-consumption-of-apis)

## Introducere

OWASP API Security Top 10 se concentrează pe vulnerabilitățile specifice API-urilor moderne. Deși shop-mvp e o aplicație web tradițională (server-side rendering), toate endpoint-urile POST sunt efectiv API-uri REST consumate de browser.

## API1: Broken Object Level Authorization

### Risc
Un utilizator poate accesa obiecte care nu-i aparțin (ex: comanda altui utilizator).

### Implementare în shop-mvp ✅

```rust
// OrderState primește doar OrderRepo + CartRepo + PaymentRepo + Auth
// — nu poate accesa products, DB direct, etc.
pub async fn orders_page(
    State(s): State<OrderState>,
    // ...
) -> Response {
    let user = match s.auth.verify_token(token_str).await {
        Ok(u) => u,
        Err(_) => return redirect_to_login(&bp),
    };
    // Verificare: user.id e injectat automat de capability-based system
    let orders = match s.orders.get_user_orders(user.id).await {
        Ok(o) => o,
        // ...
    };
}
```

### Verificare
```bash
# Încearcă să accesezi comanda altui user
curl -b "token=USER1_TOKEN" http://localhost:3001/orders
# → vezi doar comenzile userului curent
```

## API2: Broken Authentication

### Risc
Mecanism de autentificare slab care permite fraudă sau session hijacking.

### Implementare în shop-mvp ✅

```rust
// JWT cu exp claim (ASVS L2: V3.3.1)
// Account lockout (5 încercări / 15 minute)
// CSRF protection (token UUID v4)
// Rate limiting (10 req/min/IP)
```

## API3: Broken Object Property Level Authorization

### Risc
Un utilizator poate citi sau modifica proprietăți ale obiectelor la care n-ar trebui să aibă acces.

### Implementare în shop-mvp ✅

```rust
// Capability-based architecture:
// - AdminState are acces la db și payment
// - OrderState NU are acces la db direct
// - ProductState NU are acces la cart sau payment
// - AuthState NU are acces la products
// Aceasta e o garanție LA COMPILARE, nu runtime.
```

## API4: Unrestricted Resource Consumption

### Risc
API-ul permite consum nelimitat de resurse (DoS, brute force).

### Implementare în shop-mvp ✅

| Măsură | Limită |
|--------|--------|
| Rate limiting | 10 req/min/IP |
| Body limit | 2 MB max |
| Max items per order | 20 |
| Max order value | 10.000 lei |
| Session timeout | 30 min (rute sensibile) |

## API5: Broken Function Level Authorization

### Risc
Un utilizator obișnuit poate accesa funcții administrative.

### Implementare în shop-mvp ✅

```rust
// Verificare admin explicită în fiecare handler admin
pub async fn admin_products_page(
    State(s): State<AdminState>,
    headers: axum::http::HeaderMap,
    // ...
) -> Response {
    let user = match s.auth.verify_token(token_str).await {
        Ok(u) => u,
        Err(_) => return redirect_to_login(&bp),
    };
    if !user.is_admin {
        return (StatusCode::FORBIDDEN, "Acces interzis").into_response();
    }
    // ...
}
```

## API6: Unrestricted Access to Sensitive Business Flows

### Risc
API-ul permite abuzul unor flow-uri de business (ex: cumpărare repetată, fraudă cupoane).

### Implementare în shop-mvp ✅

```rust
// Idempotency check pentru plăți (previne dublarea)
fn check_idempotency(key: &str) -> Option<String> {
    get_idempotency_cache().lock().unwrap().get(key).cloned()
}

// Business logic limits
const MAX_ITEMS_PER_ORDER: usize = 20;
const MAX_ORDER_VALUE_BANI: i64 = 10_000_00;
```

## API7: Server Side Request Forgery (SSRF)

### Risc
Serverul face request-uri la destinații controlate de atacator.

### Implementare în shop-mvp ✅
- Singurul request extern e către Stripe API (URL fix: `api.stripe.com`)
- Stripe URL e hardcodat în `StripePayment`, nu din input utilizator
- Fără fetch de URL-uri din request-uri

## API8: Security Misconfiguration

### Risc
Configurări implicite nesigure, headere lipsă, erori verbose.

### Implementare în shop-mvp ✅

```rust
// Security headers middleware
async fn security_headers(req, next) -> Response {
    // HSTS
    // CSP
    // X-Frame-Options: DENY
    // X-Content-Type-Options: nosniff
    // Referrer-Policy
    // Cache-Control (rute sensibile)
}
```

## API9: Improper Inventory Management

### Risc
Endpoint-uri neutilizate sau neștiute (API-uri vechi, debug endpoints).

### Implementare în shop-mvp ⚠️ Parțial

- ✅ Toate rutele sunt listate în `main.rs` și logate la startup
- ❌ API documentation (OpenAPI/Swagger) — neimplementată
- ❌ Versioning API — toate rutele sunt `/` sau `/shop/`

### Îmbunătățire propusă

```rust
// Listare rute active la startup (deja implementat)
for route in &[
    "GET /", "GET /products", "GET /products/{slug}",
    // ...
] {
    tracing::info!("📍 {}", route);
}
```

## API10: Unsafe Consumption of APIs

### Risc
Aplicația consumă API-uri externe fără validare, timeout sau retry.

### Implementare în shop-mvp ✅

```rust
// Stripe API consumption with:
// - Retry (3 încercări cu exponential backoff)
// - Timeout (10s per request)
// - Error boundary (RetryPayment decorator)
pub struct RetryPayment {
    inner: Arc<dyn PaymentRepo>,
    max_retries: u32,
    base_delay_ms: u64,
    request_timeout_ms: u64,
}
```

## Rezumat

| API Risk | Status | Măsură principală |
|----------|--------|-------------------|
| API1: BOLA | ✅ | Capability-based state |
| API2: Broken Auth | ✅ | JWT + lockout + CSRF |
| API3: Broken Property Auth | ✅ | Compile-time garanție |
| API4: Resource Consumption | ✅ | Rate limit + body limit |
| API5: Broken Function Auth | ✅ | Verificare admin explicită |
| API6: Business Flow Abuse | ✅ | Idempotency + business limits |
| API7: SSRF | ✅ | Stripe URL hardcodat |
| API8: Misconfiguration | ✅ | Security headers |
| API9: Inventory | ⚠️ | Fără OpenAPI/Swagger |
| API10: Unsafe Consumption | ✅ | RetryPayment + timeout |
