# Arhitectura Completă a Ecosistemului Shop-MVP & MyApp

> *De la optimizarea kernelului Ubuntu și compilarea remote pe telefon până la verificarea formală inspirată de seL4 — o călătorie arhitecturală end-to-end prin toate deciziile tehnice ale unui ecosistem web modern scris în Rust.*

**Autor:** GitHub Copilot (DeepSeek V4 Flash)
**Data:** 2026-07-11
**Tag-uri:** rust, axum, arhitectura, capability-based, seL4, LEGO, zero-js, server-side, postgresql, optimizare
**Dificultate:** avansat
**Timp de citire:** 45 minute

---

## Cuprins

- [1. Privire de ansamblu — Ecosistemul complet](#1-privire-de-ansamblu--ecosistemul-complet)
- [2. Arhitectura dual-project](#2-arhitectura-dual-project)
- [3. Module LEGO — Sistemul de crate-uri reutilizabile](#3-module-lego--sistemul-de-crate-uri-reutilizabile)
- [4. Capability-based architecture (seL4-style)](#4-capability-based-architecture-sel4-style)
- [5. Filosofia zero JavaScript](#5-filosofia-zero-javascript)
- [6. Knowledge Base + AI/RAG](#6-knowledge-base--airag)
- [7. Baza de date — PostgreSQL tuning și pgvector](#7-baza-de-date--postgresql-tuning-si-pgvector)
- [8. Dezvoltare și tooling](#8-dezvoltare-si-tooling)
- [9. Build remote pe telefon (Samsung S22)](#9-build-remote-pe-telefon-samsung-s22)
- [10. Strategia de securitate pe 5 niveluri](#10-strategia-de-securitate-pe-5-niveluri)
- [11. Benchmark-uri și statistici](#11-benchmark-uri-si-statistici)
- [12. Roadmap — Faza 4 Enterprise](#12-roadmap--faza-4-enterprise)
- [13. Concluzii și lecții învățate](#13-concluzii-si-lectii-invatate)

---

## 1. Privire de ansamblu — Ecosistemul complet

Ecosistemul este compus din **două aplicații web** care împărtășesc **15 module de bază (LEGO)**, o bază de date PostgreSQL comună, și rulează pe un server desktop (BIOSTAR A10N-8800E) cu un telefon Samsung S22 folosit ca mașină de build remote.

```
┌─────────────────────────────────────────────────────────┐
│                    Ecosistemul Shop-MVP                  │
├──────────────────────┬──────────────────────────────────┤
│     myapp (3000)     │        shop-mvp (3001)           │
│                      │                                  │
│  📚 Knowledge Base   │  🛒 Coș de cumpărături          │
│  🏪 Marketplace      │  💳 Checkout + Stripe           │
│  🤖 AI/RAG API       │  📦 Comenzi + istoric           │
│  📄 SEO (sitemap,    │  🔐 Admin panel                 │
│      robots, RSS,    │  🚦 Rate limiting               │
│      JSON-LD, OG)    │  🛡️ Security headers            │
│  📡 llms.txt         │  🔄 Graceful shutdown           │
│                      │  📝 Debug logging               │
├──────────────────────┴──────────────────────────────────┤
│                    Module LEGO (libs/)                   │
│  rust-auth │ rust-cart │ rust-payment │ rust-slug       │
│  rust-url-normalizer │ rust-path-prefix │ cache         │
│  storage │ rust-marketplace-* (5x) │ u32-i32-converter  │
├─────────────────────────────────────────────────────────┤
│                    PostgreSQL (Docker)                   │
│  products │ users │ cart_items │ orders │ order_items   │
│  categories │ listings │ articles │ article_embeddings  │
│  cache (TTL)                                             │
├─────────────────────────────────────────────────────────┤
│              Infrastructură și Tooling                   │
│  mold 2.40.4 │ sccache 0.16.0 │ cargo check: 0.056s    │
│  S22 remote build: 65s (fresh) / 0.369s (incr)          │
│  PostgreSQL: shared_buffers 512MB │ random_page_cost 1.1 │
└─────────────────────────────────────────────────────────┘
```

### Tehnologii principale

| Componentă | Tehnologie | Versiune |
|------------|-----------|----------|
| Limbaj | Rust | 2024 edition |
| Web framework | Axum | 0.8.9 |
| Database | PostgreSQL (Docker) | 16 |
| ORM | SQLx | 0.9 (query-as, fără macro-uri) |
| Templating | Tera | 2.0 |
| Stilizare | TailwindCSS | CSS inline (fără CDN în prod) |
| Cache | PgCache (PostgreSQL) | Custom |
| Plăți | Stripe API direct | HTTP, nu SDK |
| Auth | JWT HttpOnly cookie | HMAC-SHA256 |
| Vector DB | pgvector | 0.8 |
| Embeddings | Ollama + nomic-embed-text | Local |
| Chat AI | DeepSeek API | Remote |

---

## 2. Arhitectura dual-project

### 2.1 De ce două aplicații?

Decizia de a separa în două binary-uri distincte (`myapp` + `shop-mvp`) a fost luată din mai multe motive:

#### Izolare prin separare fizică

```
Scenario: Un bug în handlerul de checkout face panică

Monolit:      ❌ Cade TOT — și biblioteca, și magazinul, și AI-ul
Dual-project: ✅ Cade doar shop-mvp — myapp continuă să servească articole
```

#### Izolare prin separare de securitate

```
Scenario: O vulnerabilitate în parsarea Markdown (pulldown-cmark)

Monolit:      ❌ Atacatorul poate accesa și coșul, și plățile
Dual-project: ✅ Atacatorul e limitat la myapp — cheile Stripe sunt doar în shop-mvp
```

#### Deploy independent

```
myapp (KB + Marketplace)  → se actualizează des (articole noi, embedding-uri)
shop-mvp (Magazin)        → se actualizează rar (bug-uri critice, feature-uri mari)
```

Nu are sens să faci deploy la tot monolitul când adaugi un articol.

#### Resurse separate

```
myapp:  ~50MB RAM,   ~10 req/s (conținut)
shop-mvp: ~80MB RAM, ~50 req/s (tranzacții — necesită mai multă putere)
```

Poți scala fiecare independent.

### 2.2 Punctul comun: Modulele LEGO

Ambele aplicații bootează aceleași module:

```rust
// shop-mvp/src/main.rs
let auth: Arc<dyn AuthRepo> = Arc::new(PgAuthRepo::new(pool.clone(), &jwt_secret));
let cart: Arc<dyn CartRepo> = Arc::new(PgCartRepo::new(pool.clone()));
let orders: Arc<dyn OrderRepo> = Arc::new(PgOrderRepo::new(pool.clone()));
let payment: Arc<dyn PaymentRepo> = Arc::new(RetryPayment::new(Arc::new(StripePayment::new(&stripe_secret))));
```

```rust
// myapp/src/main.rs
let cache: Arc<dyn Cache> = Arc::new(PgCache::new(pool.clone()));
// Nu are nevoie de cart, orders, payment — doar de auth și products
```

### 2.3 Rutele fiecărei aplicații

#### myapp (port 3000)

```
Feature flag: KnowledgeBase sau Marketplace sau All

📚 Knowledge Base:
  GET  /biblioteca              → listă articole
  GET  /biblioteca/{slug}       → detaliu articol (Markdown→HTML + ToC)
  GET  /cauta?q=                → search full-text
  GET  /stats                   → statistici bibliotecă
  GET  /rss.xml                 → RSS feed
  GET  /llms.txt                → context LLM
  GET  /llms-full.txt           → index complet LLM
  POST /api/chat/{slug}         → chat DeepSeek pe articol

🏪 Marketplace:
  GET  /                        → index marketplace
  GET  /categorii               → arbore categorii
  GET  /anunturi                → anunțuri (paginare)
  GET  /anunturi/{slug}         → detaliu anunț
  GET  /anunturi/nou            → formular creare anunț
  GET  /produse?brand=Apple     → catalog produse
  GET  /produs/{slug}           → detaliu produs

🤖 AI/RAG:
  GET  /api/ai/training/products     → JSONL training data
  GET  /api/ai/retrieve/products?q=  → RAG search
  GET  /api/ai/retrieve/articles/semantic?q= → pgvector search
  GET  /api/ai/openapi.json          → OpenAPI 3.1 spec
  GET  /api/ai/docs                  → HTML docs

🌐 SEO & Discovery:
  GET  /robots.txt              → permisiuni crawler-e
  GET  /sitemap.xml             → sitemap dinamic
  GET  /api/ai/search?q=        → search unificat
```

#### shop-mvp (port 3001)

```
🛒 Coș:
  GET  /cart                    → coșul curent
  POST /cart/add                → adaugă produs
  POST /cart/remove             → elimină produs

💳 Checkout:
  GET  /checkout                → formular checkout
  POST /checkout                → plasează comanda
  POST /order/{id}/pay          → plătește comanda
  GET  /orders                  → istoric comenzi
  GET  /success                 → pagină succes

🔐 Admin:
  GET  /admin                   → listă produse
  GET  /admin/orders            → listă comenzi
  POST /admin/order/{id}/status → actualizare status
  GET  /admin/product/new       → formular produs nou
  POST /admin/product/new       → creează produs
  GET  /admin/product/{slug}/edit → editare produs
  POST /admin/product/{slug}/edit → actualizare produs
  POST /admin/product/{slug}/delete → ștergere produs
  GET  /admin/logs              → debug logs

🔧 System:
  GET  /health                  → health check (cu DB)
  GET  /me                      → profil utilizator
  GET  /login                   → formular login
  POST /login                   → autentificare
  GET  /signup                  → formular înregistrare
  POST /signup                  → înregistrare
  GET|POST /logout              → deconectare
```

Ambele aplicații servesc și sub prefixul `/shop/*` (prin `Router::new().merge(routes).nest("/shop", routes)`).

---

## 3. Module LEGO — Sistemul de crate-uri reutilizabile

### 3.1 Arhitectura fiecărui modul

Fiecare modul LEGO are aceeași structură:

```
libs/rust-{nume}/
├── Cargo.toml
└── src/
    ├── lib.rs          → Trait-ul (interfața)
    └── pg.rs           → Implementarea PostgreSQL
```

### 3.2 Catalogul complet

| # | Crate | Trait | Metode principale | Locație |
|---|-------|-------|-------------------|---------|
| 1 | `rust-auth` | `AuthRepo` | `login()`, `signup()`, `verify()`, `get_user_by_id()`, `migrate()` | `libs/rust-auth/` |
| 2 | `rust-cart` | `CartRepo` | `get_cart()`, `add_item()`, `remove_item()`, `clear_cart()`, `migrate()` | `libs/rust-cart/` |
| 3 | `rust-payment` | `PaymentRepo` | `create_payment()`, `confirm_payment()`, `refund()` | `libs/rust-payment/` |
| 4 | `rust-marketplace-products` | `ProductRepo` | `list()`, `get_by_slug()`, `create()`, `update()`, `delete()`, `search()` | `libs/rust-marketplace-products/` |
| 5 | `rust-marketplace-categories` | `CategoryRepo` | `get_tree()`, `get_by_slug()`, `get_children()`, `get_breadcrumb()` | `libs/rust-marketplace-categories/` |
| 6 | `rust-marketplace-orders` | `OrderRepo` | `create()`, `get_by_user()`, `get_by_id()`, `update_status()`, `migrate()` | `libs/rust-marketplace-orders/` |
| 7 | `rust-marketplace-listings` | `ListingRepo` | `create()`, `get_all()`, `get_by_slug()`, `search()`, `increment_views()` | `libs/rust-marketplace-listings/` |
| 8 | `rust-knowledge-base` | — | Funcții pentru articles + embeddings | `libs/rust-knowledge-base/` |
| 9 | `rust-slug` | — | `generate_slug()` cu diacritice românești | `libs/rust-slug/` |
| 10 | `rust-url-normalizer` | — | Middleware: strip trailing slash | `libs/rust-url-normalizer/` |
| 11 | `rust-path-prefix` | — | Extractor Axum: `DetectBasePath` | `libs/rust-path-prefix/` |
| 12 | `cache` | `Cache` | `get()`, `set()`, `exists()`, `delete()`, `init()` + JSON helpers | `libs/cache/` |
| 13 | `storage` | `Storage` | `save()`, `load()`, `delete()`, `list()` | `libs/storage/` |
| 14 | `u32-i32-converter` | — | Conversii sigure între tipuri numerice | `libs/u32-i32-converter/` |
| 15 | `rust-wallet` | `WalletRepo` | Balance, tranzacții | `libs/rust-wallet/` |

### 3.3 Pattern-ul RetryPayment (Error Boundary)

Un exemplu de pattern avansat folosit în `rust-payment`:

```rust
/// Decorator pattern: adaugă retry + timeout peste orice implementare PaymentRepo
pub struct RetryPayment<T: PaymentRepo> {
    inner: Arc<T>,
    max_retries: u32,
    base_delay: Duration,
}

#[async_trait]
impl<T: PaymentRepo + Send + Sync> PaymentRepo for RetryPayment<T> {
    async fn create_payment(&self, amount: u32, currency: &str) -> Result<PaymentIntent, PaymentError> {
        let mut last_err = PaymentError::NetworkError("no retry".into());
        for attempt in 1..=self.max_retries {
            match self.inner.create_payment(amount, currency).await {
                Ok(intent) => return Ok(intent),
                Err(e) => {
                    last_err = e;
                    if !last_err.is_retryable() {
                        break; // Erori de validare = nu reîncerca
                    }
                    tokio::time::sleep(self.base_delay * attempt).await;
                }
            }
        }
        Err(last_err)
    }
}
```

### 3.4 Caching layer (PgCache)

Cache-ul e implementat peste PostgreSQL, cu TTL și suport JSON:

```sql
CREATE TABLE IF NOT EXISTS cache (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cache_expires ON cache(expires_at);
```

```rust
#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>>;
    async fn set(&self, key: &str, value: &str, ttl: Duration) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn init(&self) -> Result<()>;
}

// Helper pentru JSON
pub async fn get_json<T: DeserializeOwned>(cache: &dyn Cache, key: &str) -> Result<Option<T>> {
    match cache.get(key).await? {
        Some(val) => Ok(Some(serde_json::from_str(&val)?)),
        None => Ok(None),
    }
}

pub async fn set_json<T: Serialize>(cache: &dyn Cache, key: &str, val: &T, ttl: Duration) -> Result<()> {
    cache.set(key, &serde_json::to_string(val)?, ttl).await
}
```

Categoriile se cache-uiesc 30 de zile (se schimbă foarte rar). Breadcrumb-urile — la fel.

---

## 4. Capability-based architecture (seL4-style)

### 4.1 Inspirația: seL4

seL4 este un microkernel verificat formal (matematic) care funcționează pe principiul **capabilităților**: un proces nu poate accesa nimic decât dacă deține o capabilitate explicită pentru acel obiect.

Aplicat la web: un handler HTTP nu poate accesa nimic decât dacă primește explicit acel obiect prin **domain state**.

### 4.2 Implementarea în shop-mvp

```rust
// state.rs — Domain state-uri separate
//
// Fiecare structură reprezintă UN DOMENIU.
// Niciun handler nu primește AppState direct.

/// Capabilități pentru autentificare
#[derive(Clone)]
pub struct AuthState {
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}

/// Capabilități pentru produse
#[derive(Clone)]
pub struct ProductState {
    pub products: Arc<dyn ProductRepo>,
    pub renderer: RenderService,
    pub db: PgPool,  // doar pentru categorii (query simplu)
}

/// Capabilități pentru coș
#[derive(Clone)]
pub struct CartState {
    pub cart: Arc<dyn CartRepo>,
    pub products: Arc<dyn ProductRepo>,
    pub renderer: RenderService,
}

/// Capabilități pentru comenzi + plăți
#[derive(Clone)]
pub struct OrderState {
    pub orders: Arc<dyn OrderRepo>,
    pub cart: Arc<dyn CartRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
    pub site_url: String,
    pub max_qty: i32,
}

/// Capabilități pentru admin (toate)
#[derive(Clone)]
pub struct AdminState {
    pub products: Arc<dyn ProductRepo>,
    pub orders: Arc<dyn OrderRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
    pub db: PgPool,
}
```

### 4.3 Asamblarea sub-router-elor

```rust
// Fiecare sub-router are UN SINGUR tip de state
let auth_routes = Router::new()
    .route("/login", post(login_handler))
    .route("/signup", post(signup_handler))
    .with_state(AuthState { auth: state.auth.clone(), renderer: state.renderer.clone() });

let product_routes = Router::new()
    .route("/products", get(products_page))
    .route("/product/{slug}", get(product_detail_page))
    .with_state(ProductState {
        products: state.products.clone(),
        renderer: state.renderer.clone(),
        db: state.db.clone(),
    });

let cart_routes = Router::new()
    .route("/cart", get(cart_page))
    .route("/cart/add", post(cart_add))
    .with_state(CartState {
        cart: state.cart.clone(),
        products: state.products.clone(),
        renderer: state.renderer.clone(),
    });

let order_routes = Router::new()
    .route("/checkout", get(checkout_page).post(checkout_handler))
    .route("/orders", get(orders_page))
    .with_state(OrderState {
        orders: state.orders.clone(),
        cart: state.cart.clone(),
        payment: state.payment.clone(),
        auth: state.auth.clone(),
        renderer: state.renderer.clone(),
        site_url: state.site_url.clone(),
        max_qty: state.max_qty,
    });

let admin_routes = Router::new()
    .route("/admin", get(admin_products_page))
    .route("/admin/orders", get(admin_orders_page))
    .with_state(AdminState {
        products: state.products.clone(),
        orders: state.orders.clone(),
        auth: state.auth.clone(),
        renderer: state.renderer.clone(),
        db: state.db.clone(),
    });
```

### 4.4 Verificare la compilare

```rust
// ❌ Asta NU compila:
async fn checkout_handler(State(state): State<AuthState>) -> Response {
    state.payment.refund(...)?;  // ERROR: no field `payment` on `AuthState`
}
```

**Imposibil din greșeală.** Compilatorul verifică.

### 4.5 Testabilitate

```rust
#[tokio::test]
async fn test_checkout_page_empty_cart() {
    let cart = MockCartRepo::new();  // coș gol
    let orders = MockOrderRepo::new();
    let payment = MockPaymentRepo::new();
    let auth = MockAuthRepo::new();

    let state = OrderState {
        cart: Arc::new(cart),
        orders: Arc::new(orders),
        payment: Arc::new(payment),
        auth: Arc::new(auth),
        renderer: test_renderer(),
        site_url: "http://test".into(),
        max_qty: 999,
    };

    // Nu trebuie să mock-uiesc products, db, cache — nu sunt în OrderState!
    let response = checkout_page(State(state)).await;
    assert!(response.to_string().contains("Coșul e gol"));
}
```

---

## 5. Filosofia zero JavaScript

### 5.1 De ce zero JS

Proiectul a început cu HTMX 2.0.4 + module JavaScript. După **4 ore pierdute** debugging 9 bug-uri diferite (redirect loop, cookie vs localStorage, HX-Redirect race conditions, diferențe Chrome vs Firefox), s-a luat decizia radicală: **zero JavaScript în producție**.

### 5.2 PRG Pattern (Post-Redirect-Get)

```rust
async fn login_handler(
    State(state): State<AuthState>,
    Form(form): Form<LoginForm>,
) -> Response {
    match state.auth.login(&form.email, &form.password).await {
        Ok(user) => {
            let token = state.auth.generate_token(&user)?;
            // PRG: POST → 302 Redirect → GET
            (StatusCode::FOUND, [
                (header::SET_COOKIE, &format!("token={}; HttpOnly; Path=/; SameSite=Lax", token)),
                (header::LOCATION, "/"),
            ]).into_response()
        }
        Err(e) => {
            // Eroare → redirect înapoi cu ?error=
            (StatusCode::FOUND, [
                (header::LOCATION, &format!("/login?error={}", urlencode(&e.to_string()))),
            ]).into_response()
        }
    }
}
```

Fiecare form POST face 302 redirect. Fără JSON API, fără stare client-side, fără rehydratare.

### 5.3 Bug-urile care au dus la zero JS

| # | Bug | Cauză | Timp pierdut |
|---|-----|-------|-------------|
| 1 | Logout redirect la greșit | `extract_path_from_url` nu gestiona path-uri simple | ~30min |
| 2 | Admin redirect loop infinit | `localStorage.getItem('token')` în loc de HttpOnly cookie | ~20min |
| 3 | Checkout "Coșul e gol" fals | session_id necitit din cookie | ~15min |
| 4 | Login nu afișa userul | `HX-Redirect` executat înaintea scriptului de boot | ~40min |
| 5 | Login loop infinit | Referer-ul era pagina de login, nu cea originală | ~30min |
| 6 | Chrome vs Firefox comportament diferit | `Set-Cookie` + `window.location.href` race condition | ~25min |
| 7 | Redirect pierdut login↔signup | hardcoded `?redirect=` în template | ~15min |
| 8 | Nav neactualizat după HTMX | selector `a[href$="/login"]` vs `a[href*="/login"]` | ~60min |
| 9 | Parolă în URL | `hx-post` fără HTMX = GET implicit | ~10min |

**Total: ~4 ore pierdute pe bug-uri care n-ar fi existat fără JS.**

### 5.4 Security headers

```rust
async fn security_headers<B>(req: Request<B>, next: Next<B>) -> Response {
    let mut response = next.run(req).await;
    
    // CSP strict — fără script-uri inline, fără eval, doar din aceeași sursă
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        "default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'none'; img-src 'self' https: data:; font-src 'self' https://fonts.gstatic.com; connect-src 'none'; form-action 'self'".parse().unwrap(),
    );
    
    // Anti-clickjacking
    response.headers_mut().insert(
        header::X_FRAME_OPTIONS,
        "DENY".parse().unwrap(),
    );
    
    // Anti-MIME sniffing
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        "nosniff".parse().unwrap(),
    );
    
    // Referrer policy
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    
    response
}
```

### 5.5 Rate limiting

```rust
// ratelimit.rs — Rate limiter în-memory cu perioade fixe
pub struct RateLimiter {
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    pub fn check(&self, key: &str) -> Result<(), StatusCode> {
        let mut map = self.requests.lock().unwrap();
        let now = Instant::now();
        let entry = map.entry(key.to_string()).or_default();

        // Elimină intrările expirate
        entry.retain(|t| now.duration_since(*t) < self.window);

        if entry.len() >= self.max_requests {
            return Err(StatusCode::TOO_MANY_REQUESTS); // 429
        }

        entry.push(now);
        Ok(())
    }
}
```

Configurat la 10 req/min/IP pentru login/signup.

---

## 6. Knowledge Base + AI/RAG

### 6.1 Sistemul de articole

80+ articole educaționale în Markdown, scrise în limba română. Fiecare articol are:

```yaml
tag-uri: [rust, arhitectura, securitate]
dificultate: incepator | intermediar | avansat
timp de citire: 15 minute
```

Procesul de render:

```rust
pub async fn article_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let article = db::article::get_article_by_slug(&state.db, &slug)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // 1. Parsează Markdown → HTML
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&article.content, options);
    let mut html = String::new();
    html::push_html(&mut html, parser);

    // 2. Extrage cuprinsul din heading-uri
    let toc = extract_toc(&article.content);
    
    // 3. Adaugă ID-uri pentru anchor links
    let html = add_heading_ids(&html, &toc);

    // 4. Render cu Tera
    let mut ctx = Context::new();
    ctx.insert("article", &article);
    ctx.insert("content", &html);
    ctx.insert("toc", &toc);
    // ...
}
```

### 6.2 Căutarea semantică cu pgvector

```sql
-- Activare extensie
CREATE EXTENSION IF NOT EXISTS vector;

-- Tabela embeddings
CREATE TABLE article_embeddings (
    id SERIAL PRIMARY KEY,
    article_slug TEXT REFERENCES articles(slug) ON DELETE CASCADE,
    embedding vector(768),           -- nomic-embed-text: 768 dimensiuni
    chunk_text TEXT,                 -- textul chunk-ului
    chunk_index INT,                 -- poziția în articol
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index IVFFlat pentru căutare rapidă (~10x mai rapid ca scanare completă)
-- lists = sqrt(num_rows) ≈ 100 pentru ~10K embeddings
CREATE INDEX idx_embeddings_ivfflat
    ON article_embeddings
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);
```

Query-ul de căutare:

```rust
pub async fn search_articles_vector(
    db: &PgPool,
    query_embedding: &[f32; 768],
    limit: usize,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    // `<=>` = cosine distance (0 = identical, 2 = opposite)
    sqlx::query_as::<_, SearchResult>(
        r#"
        SELECT a.title, a.slug, a.summary,
               (ae.embedding <=> $1::vector) AS distance
        FROM articles a
        JOIN article_embeddings ae ON a.slug = ae.article_slug
        ORDER BY distance
        LIMIT $2
        "#
    )
    .bind(query_embedding as &[f32; 768])
    .bind(limit as i64)
    .fetch_all(db)
    .await
}
```

### 6.3 Generarea embedding-urilor cu Ollama

```rust
// bin/embed_articles.rs
async fn generate_embedding(text: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::new();
    let resp = client
        .post("http://localhost:11434/api/embeddings")
        .json(&serde_json::json!({
            "model": "nomic-embed-text",
            "prompt": text,
        }))
        .send()
        .await?
        .json::<OllamaResponse>()
        .await?;
    
    Ok(resp.embedding)  // 768 floats
}
```

Embedding-urile se generează local — **zero costuri de API extern**.

### 6.4 Chat cu DeepSeek pe articole

```rust
pub async fn chat_with_article(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(body): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    let article = db::article::get_article_by_slug(&state.db, &slug)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Construiește context din articol + întrebare
    let prompt = format!(
        "Ești un asistent specializat pe articolul următor.\n\n\
         --- Articol: {} ---\n{}\n\n\
         --- Întrebare ---\n{}\n\n\
         Răspunde doar pe baza informațiilor din articol.",
        article.title, article.content, body.question
    );

    // Cheamă DeepSeek API
    let response = call_deepseek(&state.deepseek_api_key, &prompt).await?;
    
    Ok(Json(ChatResponse { answer: response }))
}
```

### 6.5 Endpoint-uri AI Discovery

```rust
// llms.txt — sumar pentru LLM crawlers
async fn llms_txt(State(state): State<AppState>) -> Response {
    let body = format!(
        "# {} — Knowledge Base\n\n{}\n\n## Articole\n{}",
        "Biblioteca de cunoștințe",
        "Articole educaționale despre fundamentele informaticii...",
        articles.iter().map(|a| format!("- [{}]({}/biblioteca/{})", a.title, state.site_url, a.slug)).join("\n"),
    );
    cached_response(body, "text/plain; charset=utf-8", 86400)
}

// RSS feed
async fn rss_feed(State(state): State<AppState>) -> Response {
    let articles = db::article::get_all_articles(&state.db).await.unwrap_or_default();
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(r#"<rss version="2.0"><channel>"#);
    xml.push_str(&format!("<title>Biblioteca de cunoștințe</title><link>{}/</link>", state.site_url));
    for a in &articles {
        xml.push_str(&format!(
            r#"<item><title>{}</title><link>{}/biblioteca/{}</link><description>{}</description></item>"#,
            a.title, state.site_url, a.slug, a.summary,
        ));
    }
    xml.push_str("</channel></rss>");
    cached_response(xml, "application/rss+xml; charset=utf-8", 3600)
}
```

---

## 7. Baza de date — PostgreSQL tuning și pgvector

### 7.1 Configurația optimizată

```ini
# postgresql.conf — optimizat pentru SSD + dev alpha mode
shared_buffers = 512MB          # 25% din RAM (2GB), implicit era 128MB
random_page_cost = 1.1          # Pentru SSD (implicit 4.0 — pentru HDD)
work_mem = 16MB                 # 4MB → 16MB (join-uri mai rapide)
synchronous_commit = off        # Alpha mode: sacrificăm durabilitate pentru viteză
effective_cache_size = 1.5GB    # 75% din RAM
maintenance_work_mem = 128MB    # VACUUM, CREATE INDEX mai rapide
```

### 7.2 Schema principală (7 tabele)

```sql
-- Produse
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    brand TEXT NOT NULL,
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    category_id INTEGER NOT NULL DEFAULT 1,
    release_year INTEGER,
    specs JSONB NOT NULL DEFAULT '{}',
    price_new INTEGER,          -- în bani, nu FLOAT!
    affiliate_url TEXT,
    image_url TEXT,
    stock_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Categorii (ierarhice, 3 nivele)
CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    parent_id INTEGER REFERENCES categories(id),
    icon TEXT,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Utilizatori
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Coș
CREATE TABLE cart_items (
    id SERIAL PRIMARY KEY,
    session_id TEXT NOT NULL,
    product_id INTEGER REFERENCES products(id),
    quantity INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(session_id, product_id)
);

-- Comenzi
CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id),
    session_id TEXT,
    total_bani INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    payment_status TEXT NOT NULL DEFAULT 'pending',
    payment_provider TEXT DEFAULT 'stripe',
    payment_provider_id TEXT,
    shipping_name TEXT,
    shipping_address TEXT,
    shipping_phone TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Items comandă
CREATE TABLE order_items (
    id SERIAL PRIMARY KEY,
    order_id INTEGER REFERENCES orders(id),
    product_id INTEGER REFERENCES products(id),
    product_name TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    price_bani INTEGER NOT NULL
);

-- Articole (Knowledge Base)
CREATE TABLE articles (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    summary TEXT,
    content TEXT NOT NULL,
    category_path TEXT[],
    tags TEXT[],
    difficulty TEXT DEFAULT 'intermediar',
    related_concepts TEXT[],
    reading_time_minutes INTEGER,
    published_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- GIN index pentru full-text search pe articole
CREATE INDEX idx_articles_fts ON articles
    USING GIN (to_tsvector('romanian', title || ' ' || content));
```

### 7.3 De ce prețurile sunt în bani (INTEGER)

```rust
// În DB: price_new INTEGER (ex: 249900 = 2499.00 lei)
// La afișare:
let price_lei = format!("{:.2}", product.price_new as f64 / 100.0);
// "2499.00"

// La calcul:
let total = items.iter().map(|i| i.price_bani * i.quantity).sum();
// FĂRĂ floating point errors!
```

**Regula de aur:** Niciodată `FLOAT`/`DOUBLE` pentru prețuri. Întotdeauna `INTEGER` în cea mai mică unitate monetară (bani).

---

## 8. Dezvoltare și tooling

### 8.1 mold linker

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.aarch64-linux-android]
linker = "/home/iuri/.rustup/toolchains/nightly-2026-07-01-aarch64-linux-android/bin/rust-lld"

[env]
RUSTC_WRAPPER = "sccache"
```

mold 2.40.4 reduce timpul de linkare de la ~2s la ~0.2s.

### 8.2 sccache

```bash
# sccache 0.16.0 — cache distribuibil pentru compilare
# Works at the crate level: evită recompilarea crate-urilor neschimbate

# Verificare:
sccache --show-stats
# Cache size: 847 MiB
# Hits: 1247
# Misses: 89
# Hit rate: 93.3%
```

### 8.3 Optimizări Ubuntu

```bash
# CPU governor → performance (persistent)
sudo systemctl enable --now set-cpu-performance.service

# /etc/systemd/system/set-cpu-performance.service
[Service]
ExecStart=/bin/bash -c 'for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do echo performance > $cpu; done'
Type=oneshot

# swappiness 60 → 10
echo 'vm.swappiness=10' | sudo tee /etc/sysctl.d/99-swappiness.conf

# inotify 65536 → 524288 (VS Code monitorizează multe fișiere)
echo 'fs.inotify.max_user_watches=524288' | sudo tee /etc/sysctl.d/99-inotify.conf

# ZRAM: 2GB swap comprimat
sudo apt install zram-tools
echo 'PERCENT=50' | sudo tee /etc/default/zramswap
```

### 8.4 Rezultate

| Operație | Înainte | După | Îmbunătățire |
|----------|---------|------|-------------|
| `cargo check -p shop-mvp` | ~2.5s | **0.056s** | 44× |
| `cargo build -p shop-mvp` (incremental) | ~8s | **~0.5s** | 16× |
| `cargo build` (fresh, 291 crate-uri) | 10min | **65s** (S22) | 9.2× |
| PostgreSQL query (random_page_cost) | ~5ms | **~1.2ms** | 4× |
| System responsiveness | laggy | **instant** | — |

---

## 9. Build remote pe telefon (Samsung S22)

### 9.1 De ce un telefon?

| Caracteristică | Desktop (A10N-8800E) | S22 (Remote) |
|---------------|---------------------|---------------|
| **CPU** | AMD Excavator (2015, 28nm) | Snapdragon 8 Gen 1 (2022, 4nm) |
| **RAM** | 8GB DDR3-1600 | 7.1GB LPDDR5 |
| **Storage** | SATA SSD 466 MB/s | UFS 3.1 693 MB/s write, 4.5 GB/s read |
| **Build fresh (291 crate-uri)** | ~10min | **65s** |
| **Build incremental** | ~0.5s | **0.369s** |
| **Consum energie** | ~65W | ~5W |

**Paradox:** Un telefon din 2022 face build de 9× mai rapid decât un desktop din 2015, consumând de 13× mai puțină energie.

### 9.2 Configurație

```bash
# Termux pe S22:
pkg install rustc cargo git openssh
# Cross-compilare din desktop:
# scripts/build-remote.sh
ssh -p 8022 u0_a481@192.168.1.4 "cd ~/project && cargo check -p shop-mvp 2>&1"
```

### 9.3 Limitări

- **Fără sccache** pe telefon (storage limitat — 128GB)
- **Linkare mai lentă** (fără mold pe ARM Android)
- **Conexiune DB** prin rețeaua locală (192.168.1.5:5432) — ~1ms latency
- **Bateria** se descarcă în ~2h de build continuu

---

## 10. Strategia de securitate pe 5 niveluri

### Nivel 0 — Dev / Early Stage (0 Lei)

```
Stare: ACUM
Riscuri: Dacă serverul e spart, cheile sunt furate
```

- `.env` în `.gitignore`
- JWT cu secret simplu în env var
- PostgreSQL cu user/pass local
- Rate limiting 10 req/min/IP pe login/signup

### Nivel 1 — Producție mică ($0.10/lună)

```
Stare: Următorul pas
```

- Google Secret Manager în loc de `.env`
- Webhook Stripe cu verificare signature
- Input validation pe toate formularele

### Nivel 2 — Growth ($5-50/lună)

```
Stare: După primii clienți plătitori
```

- Google Cloud HSM (chei în hardware sigilat)
- JWT cu refresh token (15 min + 7 zile)
- Audit log (cine a făcut refund, șters produse)
- Stock tracking + validare

### Nivel 3 — Matur ($50-500/lună)

```
Stare: După $100K/lună
```

- Confidential VMs (RAM criptată)
- Nitro Enclave / TPM (procese sensibile separate)
- Stripe webhook signature verification

### Nivel 4 — Enterprise ($$$/lună)

```
Stare: După $1M/lună
```

- TPM hardware real (chip dedicat pe placa de bază)
- Verificare formală (seL4-inspired)
- HSM cu FIPS 140-2 Level 3

---

## 11. Benchmark-uri și statistici

### 11.1 Performanță compilare

| Măsurătoare | Desktop | S22 (remote) |
|-------------|---------|---------------|
| `cargo check` (incremental) | **0.056s** | 0.369s |
| `cargo check` (fresh) | ~45s | **4.2s** |
| `cargo build` (incremental) | **~0.5s** | ~2-3s |
| `cargo build` (fresh) | ~10min | **65s** |
| Linkare (mold) | **~0.2s** | ~1.5s (ld) |

### 11.2 Performanță sistem

| Măsurătoare | Valoare |
|-------------|---------|
| Disk read (SSD SATA) | 466 MB/s |
| Disk write (SSD SATA) | 466 MB/s |
| RAM bandwidth | 4154 ops/s |
| PostgreSQL query (simplu) | ~1.2ms |
| PostgreSQL query (pgvector, 10K rows) | ~15ms |
| HTTP response (pagina home) | ~8ms |
| HTTP response (pagina articol cu ToC) | ~25ms |

### 11.3 Statistici cod

| Măsurătoare | myapp | shop-mvp | Total |
|-------------|-------|----------|-------|
| Linii cod Rust | ~4,500 | ~3,200 | ~7,700 |
| Linii cod SQL (migrations) | ~200 | ~150 | ~350 |
| Template-uri HTML | 15 | 15 | 30 |
| Articole (Markdown) | 80+ | 17 (copiate) | 80+ |
| Module LEGO | — | — | 15 |
| Teste (template) | — | 14 | 14 |
| Teste (DB) | — | 5 | 5 |
| Bin-uri standalone | 9 | — | 9 |
| Dependințe totale | ~150 | ~120 | ~200 |

---

## 12. Roadmap — Faza 4 Enterprise

### Prioritizat (de făcut acum)

- [ ] **JSON-LD produse** — date structurate pentru Google (preț, stoc, brand)
- [ ] **Factură/Recepisă** — HTML printabil + PDF (print.js server-side)
- [ ] **Anulare comandă** — userul își poate anula comanda din UI
- [ ] **Backup DB automat** — script cron + upload la Google Cloud Storage

### Medium (după stabilizare)

- [ ] **Notificare email** — confirmare comandă, status update (SMTP)
- [ ] **Wishlist** — produse salvate de utilizator
- [ ] **Filtre căutare avansate** — preț, brand, categorie, an
- [ ] **CSRF protection** — token per formular
- [ ] **Watchdog/auto-restart** — systemd service + health check

### Complex (după producție)

- [ ] **Unificare myapp + shop-mvp** — un singur deploy cu feature flags
- [ ] **Bitcoin / BTCPay** — al doilea provider de plată
- [ ] **Cache Redis** — în loc de PgCache (mai rapid)
- [ ] **Rate limiting distribuit** — pentru mai multe instanțe

---

## 13. Concluzii și lecții învățate

### Ce a funcționat bine

1. **Arhitectura capability-based** — imposibil să faci greșeli de securitate din neatenție. Compilatorul verifică.

2. **Module LEGO cu trait-uri** — poți înlocui orice componentă fără să afectezi restul. Stripe → BTCPay = o linie schimbată.

3. **Zero JavaScript** — a eliminat 4 ore de debugging pe bug-uri care n-ar fi trebuit să existe. Paginile se încarcă în 8ms.

4. **PRG pattern peste tot** — fără "Confirm re-submit" în browser, fără dublă procesare, fără stale state.

5. **Dual-project separation** — crash în checkout? KB funcționează în continuare. Deploy de articole? Magazinul nu e afectat.

6. **Build remote pe S22** — un telefon din 2022 face build de 9× mai rapid ca un desktop din 2015. Cine ar fi crezut?

### Ce am învățat

> **"Complexitatea e inamicul securității."** — Bruce Schneier
>
> Fiecare linie de JavaScript e o oportunitate de bug.
> Fiecare handler cu `AppState` întreg e o portiță de securitate.
> Fiecare modul fără trait e o dependință greu de înlocuit.

> **"Mai puține microservicii, mai multă separare la nivel de proces."**
>
> Nu ai nevoie de Kubernetes ca să separi preocupările. Două binary-uri + systemd + reverse proxy = același efect, fără complexitatea orchestrării.

> **"Optimizează ce simți. Nu optimiza ce poți măsura."**
>
> Am redus cargo check de la 2.5s la 0.056s (44×). Utilizatorul final nu simte diferența. Am redus response time de la 30ms la 8ms. Acum paginile se simt instant. Aia contează.

> **"Toolingul e o investiție, nu o cheltuială."**
>
> Cele 2 ore investite în setup (mold, sccache, ZRAM, PostgreSQL tuning, build remote) au economisit zeci de ore în săptămânile următoare.

---

## Referințe

- [seL4 — Verificare formală a unui microkernel](/biblioteca/sel4)
- [Filosofia Hacker News în arhitectura web](/biblioteca/filosofia-hn-server-side)
- [PRG Pattern — Post-Redirect-Get în Rust+Axum](/biblioteca/prg-pattern-impl)
- [Arhitectura LEGO vs Hot Path](/biblioteca/arhitectura-lego-hotpath)
- [Strategia de securitate pe 5 niveluri](/biblioteca/strategie-securitate-nivele)
- [Cross-compilare Rust: x86_64 → ARM64](/biblioteca/cross-compilare)
- [mold + sccache — Compilare Rust 10× mai rapidă](/biblioteca/mold-sccache-advanced)
- [Ubuntu Dev Optimization](/biblioteca/ubuntu-dev-optimizare)
- [pgvector pe Android/Termux](/biblioteca/pgvector-android-termux)
- [Arhitectura Zero JavaScript](/biblioteca/arhitectura-zero-js)
- [Debugging Infinite Redirect Loop](/biblioteca/debugging-infinite-redirect-loop)
- [Frontend Debugging Journey](/biblioteca/frontend-debugging-journey)
- [Arhitectura Dual-Project: myapp + shop-mvp](/biblioteca/arhitectura-dual-project-myapp-shop-mvp)
