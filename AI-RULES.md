# AI Rules — Shop-MVP

> Trei principii. Restul decurge.

---

## Principiul 1: Tot ce intră → trece prin INPUT BOUNDARY

Browserul trimite `String`-uri. În aplicație, nu există `String`-uri brute.

```
Browser (String) → SafePath, SafeHeaders, SafeCookies → InputFactory → Email, Slug, Price, Quantity
```

**Ce înseamnă:**
- Path-ul trece prin `SafePath::parse()` în middleware
- Headerele trec prin `SafeHeaders::parse()` în middleware
- Cookie-urile trec prin `SafeCookies::parse()` în middleware
- Orice `&str` din form/query → `InputFactory::parse_*()` în handler
- Orice regulă de business → `LogicFactory::verify_*()` în handler

**Excepție:** Nu există. Chiar și în handler, nu folosești `String` brut — folosești tipuri.

```rust
// ✅ Corect
let email = InputFactory::parse_email(raw)?;  // → Email
let slug = InputFactory::parse_slug(raw)?;    // → Slug

// ❌ Greșit — ai lăsat String-ul brut să intre în aplicație
let email = raw.to_string();
```

## Principiul 2: Tot ce iese → trece prin OUTPUT BOUNDARY

Handlerul produce HTML/JSON. Înainte să ajungă la browser, trece prin output boundary.

```
Handler (SafeResponse) → security_headers_middleware (text_html, CSP, HSTS, XFO) → Browser
```

**Ce înseamnă:**
- Handlerul returnează DOAR `SafeResponse` — garantat la compilare (V7)
- Cookie-urile se setează prin `.with_cookie()` — garantate `HttpOnly; Path=/; SameSite=Lax`
- Redirect-urile se fac prin `SafeResponse::redirect()` — URL-ul trece prin `safe_redirect_url()`
- Headerele de securitate (CSP, HSTS, XFO, CTO) se adaugă AUTOMAT
- Body-ul e sanitizat prin `OutputFactory::text_html()` în middleware

```rust
// ✅ Corect
SafeResponse::redirect(url).with_cookie("token", &val, 86400)
SafeResponse::html(html)
SafeResponse::bad_request(msg)

// ❌ Greșit — ai ocolit output boundary
(StatusCode::FOUND, [("Location", url)]).into_response()
resp.headers_mut().insert(SET_COOKIE, ...)
```

## Principiul 3: Fiecare handler face DOAR ce capabilitățile lui îi permit

Handlerul primește un `State` care conține DOAR ce are nevoie.

| Handler | Capabilități (State) |
|---------|---------------------|
| auth | `AuthState` = auth |
| products | `ProductState` = products + render |
| cart | `CartState` = cart + products + auth |
| orders | `OrderState` = orders + cart + payment + auth |
| admin | `AdminState` = products + orders + payment + auth + db |

```rust
// ✅ Corect — handlerul de coș NU poate accesa OrderRepo
pub async fn cart_page(State(s): State<CartState>, ...) -> SafeResponse

// ❌ Greșit — handlerul primește prea mult
pub async fn cart_page(State(s): State<AppState>, ...) -> SafeResponse
```

---

## Reguli derivate (decrete din principii)

### Importuri
- Handler: `use crate::boundary::*;` — un singur import
- Non-handler (front_controller, trust_boundary): import direct, e în graniță

### Build
- `cargo check -p shop-mvp` pentru verificare rapidă
- `cargo build -p shop-mvp` doar când rulezi
- `test-behavior.sh` obligatoriu după ORICE modificare

### Tipuri noi
- Orice concept (email, preț, slug) → newtype, nu `String`
- Definirea în `shop-mvp/src/types/` în fișierul corespunzător

### Dependințe
- În workspace `Cargo.toml`, nu direct în crate
- `edition = "2024"`, nu `"2021"`
- `description` + `license` obligatorii

---

## Excepții cunoscute (singurele permise)

| Unde | Principiul încălcat | De ce |
|------|---------------------|-------|
| `stripe_webhook()` | #2 — returnează `impl IntoResponse` | Webhook, răspunde cu text simplu |
| `admin_migrate_orders()` | #2 — returnează `Json` | API JSON, nu HTML |
| `front_controller.rs` | #1 — importă direct OutputFactory | E chiar granița |
| `handlers/products.rs` | #1 — importă direct Html/StatusCode | Acolo e definit render_safe_json (helper, nu handler direct) |
