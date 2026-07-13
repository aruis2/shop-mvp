# AI Rules — Shop-MVP

> Respectă aceste reguli sau codul tău va fi respins la compilare sau testare.
> Acest fișier e conceput pentru AI. Regulile sunt binare: le respecți sau nu.

---

## 1. Tipuri returnate — handlere

- REGULA: Orice handler public → `SafeResponse`. NICIODATĂ `impl IntoResponse`.
- REGULA: Orice handler public → NICIODATĂ `Response`, `Html<String>`, `(StatusCode, String)`.
- REGULA: Orice handler public → NICIODATĂ `.into_response()`.
- REGULA: Dacă vezi un handler care returnează `impl IntoResponse`, **trebuie schimbat**.
- REGULA: Dacă vezi `.into_response()` într-un handler, **trebuie eliminat**.

```rust
// ✅ Corect
pub async fn home_page(...) -> SafeResponse {
    render_safe_json(...).await
}

// ❌ Greșit
pub async fn home_page(...) -> impl IntoResponse { ... }
pub async fn home_page(...) -> Response { ... }
pub async fn home_page(...) -> Result<Html<String>, (StatusCode, String)> { ... }
```

## 2. SafeResponse — construcție

- REGULA: Cookie-urile se manipulează DOAR prin `.with_cookie()` / `.without_cookie()`.
- REGULA: Headerele se adaugă DOAR prin `.with_header()`.
- REGULA: NICIODATĂ `resp.headers_mut().insert(...)`.
- REGULA: NICIODATĂ `(StatusCode::FOUND, [("Location", url)]).into_response()`.
- REGULA: NICIODATĂ `(StatusCode::BAD_REQUEST, msg).into_response()`.

```rust
// ✅ Corect
SafeResponse::redirect(url).with_cookie("token", &val, 86400)
SafeResponse::html(html).with_header("X-Custom", "val")
SafeResponse::bad_request(msg)

// ❌ Greșit
(StatusCode::FOUND, [("Location", url)]).into_response()
resp.headers_mut().insert(SET_COOKIE, ...)
```

## 3. Importuri — handlere

- REGULA: Handlerele importă DOAR din `crate::boundary::*`.
- REGULA: NICIODATĂ `use crate::types::*` direct în handler.
- REGULA: NICIODATĂ `use crate::cookie::*` direct în handler.
- REGULA: NICIODATĂ `use crate::front_controller::*` direct în handler.
- REGULA: NICIODATĂ `use crate::trust_boundary::*` direct în handler.
- REGULA: NICIODATĂ `use axum::http::StatusCode` direct în handler.
- REGULA: NICIODATĂ `use axum::response::{Html, IntoResponse, Response}` în handler.

```rust
// ✅ Corect — în handler
use crate::boundary::*;

// ❌ Greșit — în handler
use crate::types::InputFactory;
use crate::cookie::get_cookie;
use axum::http::StatusCode;
use axum::response::Html;
```

## 4. Importuri — module interne

- REGULA: `front_controller.rs` poate importa direct `crate::trust_boundary as tb`.
- REGULA: `front_controller.rs` poate importa direct `crate::types::output::OutputFactory`.
- REGULA: `handlers/products.rs` (sursa pentru `render_safe_json`) poate importa `axum::response::Html` și `axum::http::StatusCode` — e singura excepție.

## 5. Stil handler

- REGULA: NICIODATĂ `return Ok(html)` sau `return Err(...)` — handlerul nu returnează `Result`.
- REGULA: Early return cu `return SafeResponse::redirect(...)`.
- REGULA: `?` NU se folosește în handlere (nu mai e `Result`).

```rust
// ✅ Corect
let product = match s.products.get_by_slug(&slug).await {
    Ok(Some(p)) => p,
    Ok(None) => return SafeResponse::not_found(),
    Err(e) => return SafeResponse::server_error(e.to_string()),
};

// ❌ Greșit
let product = s.products.get_by_slug(&slug).await
    .map_err(|e| (StatusCode::NOT_FOUND, e))?
    .ok_or((StatusCode::NOT_FOUND, "missing"))?;
```

## 6. Randare

- REGULA: Handlerele folosesc `render_safe_json()` sau `render_safe()`, nu `render_or_err_json()`.
- REGULA: `render_safe_json()` e în `crate::handlers::products::render_safe_json`.
- REGULA: NICIODATĂ `render_or_err_json().await` în handler.

```rust
// ✅ Corect
render_safe_json(&s.renderer, "template.html", &data, &bp, &headers, &*s.auth).await

// ❌ Greșit
render_or_err_json(&s.renderer, "template.html", &data, &bp, &headers, &*s.auth).await
```

## 7. Capability — State-uri

- REGULA: Fiecare handler primește DOAR State-ul specific domeniului său.
- REGULA: `AuthState` → auth. `CartState` → cart + products + auth. `OrderState` → orders + cart + payment + auth. `AdminState` → tot.
- REGULA: NICIODATĂ `State(s): State<AppState>` într-un handler.
- REGULA: Dacă handlerul are nevoie de `OrderRepo`, trebuie să primească `OrderState`, nu `AppState`.

```rust
// ✅ Corect
pub async fn cart_page(State(s): State<CartState>, ...) -> SafeResponse

// ❌ Greșit
pub async fn cart_page(State(s): State<AppState>, ...) -> SafeResponse
```

## 8. Build și testare

- REGULA: `cargo check -p shop-mvp` — verificare rapidă. Folosește asta, nu `cargo build`.
- REGULA: `cargo build -p shop-mvp` — doar când trebuie să rulezi efectiv.
- REGULA: După ORICE modificare, rulează `test-behavior.sh` (cu serverul pe portul 3001).
- REGULA: Dacă `test-behavior.sh` trece, codul e gata. Nu mai verifica manual.
- REGULA: NICIODATĂ `cargo build --release` în development.
- REGULA: Serverul trebuie să ruleze pe portul 3001 ÎNAINTE de test-behavior.sh.
- REGULA: După modificări: `pkill -f shop-mvp` apoi `cargo run -p shop-mvp &`.

## 9. Dependințe

- REGULA: Orice dependență nouă se adaugă în workspace `Cargo.toml`, nu direct în crate.
- REGULA: NICIODATĂ `edition = "2021"` — doar `2024`.
- REGULA: Orice crate din workspace trebuie să aibă `description` și `license` în `Cargo.toml`.

## 10. Securitate — input

- REGULA: Inputul se validează prin `InputFactory::parse_*()` — NICIODATĂ validare manuală.
- REGULA: Orice `String` de la browser trece prin `InputFactory` înainte de a fi folosit.
- REGULA: Regulile de business se verifică prin `LogicFactory::verify_*()`.
- REGULA: URL-urile de redirect trec prin `OutputFactory::safe_redirect_url()`.

```rust
// ✅ Corect
let email = InputFactory::parse_email(raw)?;
let slug = InputFactory::parse_slug(raw)?;
LogicFactory::verify_ownership(&user.id, &order.user_id, "order")?;
let safe_url = OutputFactory::safe_redirect_url(&dest, "/");

// ❌ Greșit
let email = raw.to_string(); // string brut!
if user.role == "admin" { ... } // verificare manuală
```

## 11. Securitate — output

- REGULA: OutputFactory::text_html() se aplică AUTOMAT în security_headers_middleware.
- REGULA: OutputFactory::safe_error_msg() se folosește pentru mesaje de eroare în redirect-uri.
- REGULA: SafeResponse adaugă AUTOMAT: CSP, HSTS, XFO, CTO, Referrer-Policy.
- REGULA: Nu adăuga manual headerele de securitate — SafeResponse le adaugă singur.

## 12. Tipuri noi

- REGULA: Orice concept nou din domeniu (email, preț, cantitate) → newtype, nu String.
- REGULA: Newtype-urile se definesc în `shop-mvp/src/types/` în fișierul corespunzător.
- REGULA: Orice newtype trebuie să implementeze `std::fmt::Display`.
- REGULA: Orice newtype care se serializează trebuie să aibă `serde::Serialize`.

```rust
// ✅ Corect
pub struct Email(String);
pub struct Price(i64);

// ❌ Greșit
fn create_user(email: String) { ... } // ce e String-ul ăsta? email? nume? adresă?
```

---

## NOTĂ: Excepții cunoscute

Acestea sunt SINGURELE încălcări acceptate ale regulilor de mai sus:

| Fișier | Regulă încălcată | Motiv |
|--------|------------------|-------|
| `stripe_webhook()` | Returnează `impl IntoResponse`, nu `SafeResponse` | Webhook-ul Stripe răspunde cu text simplu |
| `admin_migrate_orders()` | Returnează `Result<Json<...>, ...>`, nu `SafeResponse` | JSON API |
| `front_controller.rs` | Importă direct `crate::types::output::OutputFactory` | E în graniță, nu în handler |
| `handlers/products.rs` | Importă `axum::response::Html` și `axum::http::StatusCode` | Acolo e definit `render_or_err` / `render_or_err_json` (nu se mai folosesc) |
