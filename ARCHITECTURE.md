# 🏗️ Arhitectura Shop MVP

> Un sistem web securizat prin 3 fabrici, capability-based LEGO modules,
> și defense in depth — inspirat de seL4, HN philosophy, și OWASP ASVS.
>
> Rust 1.96 · Axum 0.8 · Tera 2.0 · SQLx 0.9 · PostgreSQL 18

---

## 1. Filosofia arhitecturală

### 1.1 Trust Boundary — totul e la graniță

```
Outside World (necontrolat)
       │
       ▼
┌─────────────────────────────┐
│     STRATUL 1: INPUT        │  ← InputFactory + parser.rs
│     (validare tip + format) │
├─────────────────────────────┤
│     STRATUL 2: BUSINESS     │  ← LogicFactory
│     (reguli — e permis?)    │
├─────────────────────────────┤
│     TRUSTED ZONE            │  ← date garantat valide + permise
│                             │
│     Handlere · LEGO · DB    │
│                             │
├─────────────────────────────┤
│     STRATUL 3: OUTPUT       │  ← OutputFactory
│     (sanitizare — e sigur?) │
└─────────────────────────────┘
       │
       ▼
Client (browser, curl, etc.)
```

### 1.2 Principii

| # | Principiu | Explicație |
|---|-----------|------------|
| 1 | **Parse, don't validate** | Datele se transformă în tipuri sigure (`Email`, `Price`) — nu se validează string-uri |
| 2 | **Zero intermediaries** | Nu există string-uri nesigure între input și factory |
| 3 | **Capability-based** | Fiecare handler are doar ce-i trebuie (AuthState, CartState, etc.) |
| 4 | **Defense in depth** | 8 straturi de apărare, nu doar unul |
| 5 | **Fail fast** | Orice input invalid → eroare imediat, nu mai departe |
| 6 | **LEGO modules** | Fiecare modul e un crate independent cu trait propriu |
| 7 | **HN Philosophy** | Zero JS, minimal dependencies, own parser |
| 8 | **Test at boundary** | Fabrica e testată, nu handler-ele |

---

## 2. Cele 3 fabrici

### 2.1 InputFactory — granița de intrare

**Locație**: `shop-mvp/src/types/mod.rs`
**Metode**: 17
**Teste**: 19 (prin tipurile individuale)

```rust
pub struct InputFactory;

impl InputFactory {
    pub fn parse_email(s: &str) -> Result<Email, InputError>;
    pub fn parse_price(bani: i32) -> Result<Price, InputError>;
    pub fn parse_qty(val: i32) -> Result<Quantity, InputError>;
    pub fn parse_phone(s: &str) -> Result<PhoneNumber, InputError>;
    pub fn parse_slug(s: &str) -> Result<Slug, InputError>;
    pub fn parse_session_id(s: &str) -> Result<SessionId, InputError>;
    pub fn parse_user_id(s: &str) -> Result<UserId, InputError>;
    pub fn parse_order_id(s: &str) -> Result<OrderId, InputError>;
    pub fn parse_product_id(val: i32) -> Result<ProductId, InputError>;
    pub fn parse_category_id(val: i32) -> Result<CategoryId, InputError>;
    pub fn parse_name(s: &str) -> Result<ShippingName, InputError>;
    pub fn parse_address(s: &str) -> Result<ShippingAddress, InputError>;
    pub fn parse_notes(s: &str) -> Result<Notes, InputError>;
    pub fn parse_brand(s: &str) -> Result<Brand, InputError>;
    pub fn parse_product_name(s: &str) -> Result<ProductName, InputError>;
    pub fn parse_search(s: &str) -> Result<SearchQuery, InputError>;
    pub fn parse_currency(s: &str) -> Result<Currency, InputError>;
}
```

**Tipurile returnate** — sînt **no-type wrappers** cu constructori **privați**:
- `Email(String)` — garantat valid, lowercase, cu @ și domeniu
- `Price(i32)` — garantat între 1 și 10_000_000
- `Quantity(u32)` — garantat între 1 și 999
- `Slug(String)` — garantat alphanumeric + cratime, lowercase
- `PhoneNumber(String)` — garantat 10 cifre, stripped non-digits

Singura cale de a crea unul: `InputFactory::parse_*()`.

### 2.2 LogicFactory — regula de business

**Locație**: `shop-mvp/src/types/logic.rs`
**Metode**: 10
**Teste**: 23

```rust
pub enum LogicError {
    Forbidden,                    // IDOR
    Unauthorized(String),        // Rol insuficient
    InvalidStatus(String),       // Tranziție stare invalidă
    InsufficientStock(i32, i32), // Stoc insuficient
    LimitExceeded(String),       // Depășire limită
    NotFound(String),            // Resursă inexistentă
    Duplicate(String),           // Operație duplicat
    Other(String),               // Altă eroare
}

pub struct LogicFactory;

impl LogicFactory {
    // Ownership (IDOR prevention)
    pub fn verify_ownership<T: Eq>(user_id: &T, owner_id: &T, object: &str) -> Result<(), LogicError>;

    // Authorization
    pub fn verify_admin(role: &str) -> Result<(), LogicError>;
    pub fn verify_role(role: &str, required: &str) -> Result<(), LogicError>;

    // State machine
    pub fn verify_not_paid(payment_status: &str) -> Result<(), LogicError>;
    pub fn verify_status_transition(current: &str, next: &str) -> Result<(), LogicError>;

    // Business rules
    pub fn verify_stock_available(stock: i32, requested: i32) -> Result<(), LogicError>;
    pub fn verify_qty_in_range(qty: i32, min: i32, max: i32) -> Result<(), LogicError>;
    pub fn verify_max_value(value: i64, max: i64, label: &str) -> Result<(), LogicError>;
    pub fn verify_found<T>(resource: Option<T>, name: &str) -> Result<T, LogicError>;
    pub fn verify_not_duplicate(already_done: bool, msg: &str) -> Result<(), LogicError>;
}
```

**State machine de comenzi:**

```
pending ──→ confirmed ──→ shipped ──→ delivered
   │             │
   └──→ cancelled┘
                    paid ──→ refunded
                    unpaid ──→ paid (doar prin webhook Stripe)
```

### 2.3 OutputFactory — granița de ieșire

**Locație**: `shop-mvp/src/types/output.rs`
**Metode**: 10
**Teste**: 39

```rust
pub struct OutputFactory;

impl OutputFactory {
    // XSS prevention
    pub fn html_encode(s: &str) -> String;        // & < > " ' → entități HTML
    pub fn text_html(s: &str) -> String;           // text sigur pentru HTML

    // Error safety
    pub fn safe_error_msg(msg: &str) -> String;    // fără control chars, max 200

    // Open redirect prevention
    pub fn safe_redirect_url(url: &str, site_url: &str) -> Option<String>;
    // Blochează: javascript:, data:, vbscript:, file:, blob:, //

    // HTTP response splitting prevention
    pub fn safe_header_value(val: &str) -> String; // fără \r \n, max 1000
    pub fn safe_cookie_value(val: &str) -> String; // fără ; , space, max 256

    // Tera context sanitization
    pub fn sanitize_context(ctx: &mut Context);    // walk recursiv, html_encode
    pub fn make_context(data: &Value) -> Context;  // JSON → Context sanitizat

    // Typed display methods
    pub fn email_html(email: &str) -> String;
    pub fn price_lei(price: &Price) -> String;
    pub fn phone_display(phone: &str) -> String;
    pub fn slug_url(slug: &str) -> String;
    pub fn quantity_display(qty: &Quantity) -> String;
    pub fn currency_display(currency: &Currency) -> String;
}
```

### 2.4 QueryValidator — validare query params

**Locație**: `shop-mvp/src/types/mod.rs`
**Metode**: 5

```rust
pub struct QueryValidator;

impl QueryValidator {
    pub fn page(val: Option<i64>, default: i64) -> i64;     // loghează page invalid
    pub fn uuid(val: Option<String>, name: &str) -> Option<Uuid>;
    pub fn token(val: Option<String>, name: &str) -> Option<String>; // format JWT
    pub fn session_id(val: Option<String>, name: &str) -> Option<String>; // max 256
    pub fn header(val: Option<String>, name: &str, max_len: usize) -> Option<String>;
}
```

---

## 3. LEGO Modules (crate-uri independente)

Fiecare modul e un crate separat cu:
- Propriul **trait** (e.g., `CartRepo`, `OrderRepo`)
- Propria **implementare PostgreSQL** (`PgCartRepo`)
- Propriul **error type** (e.g., `CartError`, `OrderError`)
- **Zero dependințe** pe alte crate-uri din proiect
- **Testable** cu mock-uri

```
libs/
├── rust-auth/                  ← AuthRepo trait + PgAuthRepo
├── rust-cart/                  ← CartRepo trait + PgCartRepo
├── rust-marketplace-orders/    ← OrderRepo trait + PgOrderRepo
├── rust-marketplace-products/  ← ProductRepo trait + PgProductRepo
├── rust-marketplace-listings/  ← ListingRepo trait
├── rust-marketplace-categories/← CategoryService trait
├── rust-payment/               ← PaymentRepo trait (Stripe HTTP)
├── rust-wallet/                ← WalletRepo trait
├── storage/                    ← StorageRepo trait
├── rust-knowledge-base/        ← KnowledgeBase trait
├── cache/                      ← Cache layer
├── rust-slug/                  ← Slug generation utility
├── rust-path-prefix/           ← URL path prefix handling
├── rust-url-normalizer/        ← URL normalization
└── u32-i32-converter/          ← Type conversion utility
```

### 3.1 Capability-based State

```rust
// Fiecare handler primește DOAR ce are nevoie
struct AuthState { auth: Arc<dyn AuthRepo>, renderer: RenderService, site_url: String }
struct CartState { cart: Arc<dyn CartRepo>, products: Arc<dyn ProductRepo>, auth: Arc<dyn AuthRepo>, renderer: RenderService, max_qty: i32 }
struct ProductState { products: Arc<dyn ProductRepo>, renderer: RenderService, auth: Arc<dyn AuthRepo> }
struct OrderState { orders: Arc<dyn OrderRepo>, cart: Arc<dyn CartRepo>, auth: Arc<dyn AuthRepo>, payment: Arc<dyn PaymentRepo>, renderer: RenderService, site_url: String }
struct AdminState { products: Arc<dyn ProductRepo>, orders: Arc<dyn OrderRepo>, auth: Arc<dyn AuthRepo>, payment: Arc<dyn PaymentRepo>, renderer: RenderService, db: PgPool }
```

---

## 4. Flow-ul unui request (exemplu complet)

### 4.1 Adăugare în coș (`POST /cart/add`)

```
Browser
  │ POST /cart/add
  │ product_slug=samsung-s22&qty=1
  │ Cookie: session_id=abc-123
  ▼
┌──────────────────────────────────────────────────────────────────────┐
│ STRATUL 1 — INPUT FACTORY                                            │
│                                                                      │
│  body = "product_slug=samsung-s22&qty=1"  (RAW, neverificat)         │
│       │                                                              │
│  parser::parse_any_into(body)                                        │
│       │  desparte după & și =, URL-decodează                         │
│       │  → [FormField("product_slug","samsung-s22"),                 │
│       │     FormField("qty","1")]                                    │
│       │                                                              │
│  InputFactory::parse_slug("samsung-s22")                             │
│       │  → Slug("samsung-s22")  ✅ (lowercase, alphanumeric, cratime)│
│       │                                                              │
│  InputFactory::parse_qty(1)                                          │
│       │  → Quantity(1)  ✅ (între 1 și 999)                          │
│       │                                                              │
│  cookie = get_cookie("session_id")                                   │
│       │  → "abc-123"                                                 │
└──────┼───────────────────────────────────────────────────────────────┘
       │ date garantat valide: Slug, Quantity, session_id string
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ STRATUL 2 — LOGIC FACTORY                                            │
│                                                                      │
│  LogicFactory::verify_qty_in_range(qty=1, min=1, max=99)             │
│       │  → OK ✅                                                     │
│                                                                      │
│  LogicFactory::verify_stock_available(stock=10, qty=1)               │
│       │  → OK ✅                                                     │
└──────┼───────────────────────────────────────────────────────────────┘
       │ operație permisă
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ TRUSTED ZONE                                                         │
│                                                                      │
│  CartRepo::add_item("abc-123", AddCartItemRequest {                  │
│      product_slug: "samsung-s22",                                    │
│      product_name: "Samsung S22",                                    │
│      price_bani: 249900,                                             │
│      qty: 1,                                                         │
│  })                                                                  │
│       │                                                              │
│  SQL: INSERT INTO cart_items (...) VALUES (...)                       │
│       ON CONFLICT (session_id, product_slug, price_bani)             │
│       DO UPDATE SET qty = cart_items.qty + EXCLUDED.qty              │
│       │  → row added/updated in DB                                   │
│                                                                      │
│  set_cookie("session_id", "abc-123", 30d)                            │
│       │  → "session_id=abc-123; HttpOnly; Max-Age=2592000"          │
└──────┼───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ STRATUL 3 — OUTPUT FACTORY                                           │
│                                                                      │
│  safe_redirect_url("/products", "/")                                 │
│       │  → "/products"  ✅ (relative path, safe)                      │
│                                                                      │
│  safe_cookie_value("abc-123")                                        │
│       │  → "abc-123"  ✅ (fără caractere periculoase)                │
└──────┼───────────────────────────────────────────────────────────────┘
       │
       ▼
Browser
  ← 302 Found
  ← Location: /products
  ← Set-Cookie: session_id=abc-123; HttpOnly; Max-Age=2592000
```

### 4.2 Checkout + plată (`POST /checkout` → Stripe → webhook)

```
Browser ── POST /checkout
   │ session_id, shipping_name, address, phone, email
   ▼
InputFactory ── parse_session_id, parse_name, parse_address, parse_phone, parse_email
   │
   ▼
LogicFactory ── verify_stock_available (la fiecare produs)
   │
   ▼
OrderRepo::place_order
   │  BEGIN TRANSACTION
   │  SELECT stock_count FROM products WHERE slug = $1 FOR UPDATE  ← blochează rîndul
   │  UPDATE products SET stock_count = stock_count - $1           ← decrementează
   │  INSERT INTO orders ...
   │  INSERT INTO order_items ...                                  ← itemii
   │  COMMIT
   │
   ▼
PaymentRepo::create_checkout ── Stripe API ── checkout_url
   │
   ▼
OutputFactory ── safe_redirect_url(checkout_url)
   │
Browser ── 302 → Stripe checkout page
   │
   │ (mai tîrziu) Stripe webhook ── POST /stripe-webhook
   │   │  Verificare semnătură (x-stripe-webhook-type)
   │   │  Idempotency check (previne dublarea)
   │   │  UPDATE orders SET payment_status = 'paid'
   │
   ▼
Browser ── GET /success?order_id=xxx
   │  NU mai marchează paid — doar afișează pagina
   │  (Stripe webhook-ul e singurul care confirmă)
```

---

## 5. Baza de date

### 5.1 Tabele principale

```sql
-- users (prin rust-auth)
users (id, email, password_hash, name, role, created_at)

-- products (prin rust-marketplace-products)
products (id, brand, name, slug UNIQUE, category_id, release_year,
          specs JSONB, price_new, affiliate_url, image_url,
          stock_count DEFAULT 10, created_at)

-- categories
categories (id, name, slug)

-- cart_items
cart_items (id, session_id, user_id?, product_slug, product_name,
            price_bani, qty, created_at, updated_at)
UNIQUE (session_id, product_slug, price_bani)  -- previne duplicate

-- orders
orders (id, user_id?, session_id, guest_email?, status, payment_status,
        total_bani, shipping_name, shipping_address, shipping_phone, notes,
        payment_provider?, payment_provider_id?, created_at, updated_at)

-- order_items
order_items (id, order_id FK, product_slug, product_name, price_bani, qty)
```

### 5.2 Tranzacții și concurență

| Operație | Protecție |
|----------|-----------|
| **Cart add** | `INSERT ... ON CONFLICT DO UPDATE` + UNIQUE constraint |
| **Product update** | `SELECT ... FOR UPDATE` în tranzacție |
| **Place order** | `SELECT stock_count ... FOR UPDATE` + decrement în tranzacție + commit atomic |
| **Stripe webhook** | Idempotency cache (Mutex<HashMap>) — previne procesarea duplicat |

---

## 6. Mecanisme de securitate (defense in depth)

```
Strat 1:  InputFactory        ← parsează + validează tipul (sintaxă)
Strat 2:  QueryValidator      ← loghează query params invalide
Strat 3:  LogicFactory        ← verifică permisiunile (semantică)
Strat 4:  LEGO modules        ← capability-based (nu poți ce nu ai)
Strat 5:  Tranzacții          ← SELECT FOR UPDATE, INSERT ON CONFLICT
Strat 6:  OutputFactory       ← sanitizează tot ce iese
Strat 7:  Tera auto-escape    ← template-level XSS protection
Strat 8:  CSP headers         ← script-src, frame-ancestors, object-src
```

### 6.1 Atacuri prevenite

| Atac | Prevenit de |
|------|-------------|
| XSS (reflected + stored) | OutputFactory + Tera + CSP |
| XSS (context JavaScript) | `html_encode()` + CSP `script-src 'self'` |
| Open redirect | `safe_redirect_url()` — blochează javascript:, data:, // |
| HTTP response splitting | `safe_header_value()` — elimină \r \n |
| SQL injection | SQLx query_as (parametrized queries) |
| IDOR | `LogicFactory::verify_ownership()` |
| Privilege escalation | `LogicFactory::verify_admin()` + capability state |
| Payment double-charge | `verify_not_paid()` + idempotency cache |
| Invalid state transition | `verify_status_transition()` |
| Race condition (cart) | `INSERT ON CONFLICT` + UNIQUE constraint |
| Lost update (product) | `SELECT FOR UPDATE` în tranzacție |
| Overselling (checkout) | `SELECT stock FOR UPDATE` + decrement atomic |
| Clickjacking | CSP `frame-ancestors 'none'` |
| Fake payment confirmation | Stripe webhook e SINGURUL care marchează `paid` |
| Brute force login | Ratelimiter (10/min/IP) + account lockout (5/15min) |
| Mass assignment | InputFactory — nu poți pasa cîmpuri nedefinite |
| Parameter pollution | Parser propriu (`parser.rs`) — nu serde_urlencoded |
| Query param injection | `QueryValidator` + `InputFactory::parse_search()` |
| Parolă slabă | OWASP V2.1: lungime 8-128, literă mare, mică, cifră |

---

## 7. Testare

### 7.1 Teste unitare (fabrica)

```
types/output.rs:   39 tests  ← OutputFactory (fiecare metodă + edge cases)
types/logic.rs:    23 tests  ← LogicFactory (ownership, admin, state machine)
types/parser.rs:    8 tests  ← parser.rs (URL decode, form parse)
types/email.rs:     7 tests  ← Email::parse
types/price.rs:     6 tests  ← Price::new, Price::total
types/phone.rs:     4 tests  ← PhoneNumber::parse
types/slug.rs:      5 tests  ← Slug::parse
types/quantity.rs:  -        ← Quantity::new
types/currency.rs:  5 tests  ← Currency::parse
types/id_types.rs:  3 tests  ← SessionId, UserId, OrderId
```

### 7.2 Teste de integrare (DB)

```
shop-mvp  +  DB reală (PostgreSQL 18)
├── test_cart              — adăugare, increment, ștergere itemi
├── test_checkout          — checkout cu produse reale
├── test_login / signup    — auth complet
├── test_orders            — creare comandă, listare
├── test_products          — listare, paginare, filtrare
├── test_search            — căutare cu/prin rezultate
├── test_admin_*           — admin CRUD produse, comenzi
└── test_success           — pagină success
```

### 7.3 Teste curl (HTTP)

`test-curl.sh` — ~100 de cazuri care verifică status codes:
```
Pagini generale     200  ← GET /, /health, /login, /signup
Auth                302  ← login reușit, 400 pe erori
Produse             200  ← listare, paginare, search
Coș                 302  ← add, remove, erori
Comenzi             302  ← checkout, orders, success
Admin               200  ← fără auth (redirect HTML), 200/302 cu auth
Rute inexistente    404  ← /nonexistent, /admin/nonexistent
Trailing slash      301  ← /products/ → /products
```

### 7.4 Teste de securitate

```bash
cargo audit          # vulnerabilități în dependințe
cargo clippy -D warnings  # lint strict
```

---

## 8. Logging și observabilitate

### 8.1 Log-uri de securitate

```
WARN  logic::idor: IDOR încercat: user=... owner=... object=order
WARN  query: page invalid: -1 (folosesc default 1)
WARN  query: token invalid format: abc (ignorat)
WARN  query: session_id suspect: 300 caractere (ignorat)
WARN  header: x-session-id conține control chars (ignorat)
WARN  ratelimit: Rate limit depășit pentru 192.168.1.4
WARN  auth::ratelimit: Rate limit signup de la IP=...
WARN  cart::add: InputFactory: InvalidSlug("...")
ERROR orders::stripe: Stripe checkout eșuat: ...
INFO  stripe::webhook: Eveniment Stripe: checkout.session.completed
INFO  stripe::webhook: ✅ Plată confirmată pentru comanda ...
INFO  orders::success: Pagină success pentru comanda ...
```

### 8.2 Mediu

```
RUST_LOG=debug       ← logging complet
RUST_BACKTRACE=full  ← stack trace la panică
mold linker          ← linking rapid
sccache              ← cache compilare
```

---

## 9. Dependințe externe (minimal)

```
axum 0.8             ← web framework
sqlx 0.9             ← PostgreSQL driver (query_as, fără macros)
tera 2.0             ← template engine
tokio 1.52           ← async runtime
serde 1 / serde_json ← serializare (doar pentru JSON API)
uuid 1               ← UUID generation
chrono 0.4           ← date/time
tracing 0.1          ← logging structurat
async-trait 0.1      ← async traits in modules
```

**Zero dependințe pentru parsing**: `parser.rs` e 100% custom (120 linii).

---

## 10. Mediul de develop

```
OS:        Arch Linux (kernel 6.x)
Editor:    VS Code
DB:        PostgreSQL 18 (pgvector) în Docker
Remote:    S22 (Termux) la 192.168.1.4:8022
Linker:    mold 2.40
Cache:     sccache 0.16
Test:      cargo test + cargo clippy + cargo audit
```

---

## 11. Git history (commits relevante)

```
docs: TRUST-BOUNDARY.md actualizat — toate cele 3 fabrici + QueryValidator
fix: 4 bug-uri critice — success_page, stock decrement, FOR UPDATE, password
fix: header-e validate prin QueryValidator — x-session-id
fix: InputFactory validatează și query params — ?q=
fix: QueryValidator — query params invalide nu mai sînt ignorate tăcut
feat: LogicFactory + InputFactory in all handlers + tranzacții FOR UPDATE
feat: OutputFactory completă — toate căile de ieșire acoperite
fix: XSS sinks + OutputFactory integration în auth și admin
feat: OutputFactory in all handlers — sanitizare automată
fix: redirect + error helpers prin OutputFactory
fix: CSP întărit — producție-ready
feat: InputFactory (parser.rs) + OutputFactory
docs: PHILOSOPHY #15 + STANDARDS #63 — Rust gap
```

---

## 12. Metrici

| Metrică | Valoare |
|---------|---------|
| **Linii de cod** (Rust) | ~8000 |
| **Crate-uri** | 15 LEGO modules + 1 binary |
| **Teste unitare** | ~150 (InputFactory 19 + LogicFactory 23 + OutputFactory 39 + altele) |
| **Teste integrare** | 16 (cu DB reală) |
| **Teste curl** | ~100 (HTTP status codes) |
| **0 vulnerabilities** | cargo audit |
| **0 warnings** | cargo clippy -D warnings |
| **0 unsafe** | în producție |
