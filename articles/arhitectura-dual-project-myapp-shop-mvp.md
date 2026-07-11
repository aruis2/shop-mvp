# Arhitectura Dual-Project: De ce un ecosistem are nevoie de două aplicații

> *Cum am structurat un proiect web complex în două aplicații complementare — una pentru conținut și descoperire, alta pentru tranzacții și administrare — folosind un sistem de module LEGO reutilizabile și o arhitectură capability-based inspirată de seL4.*

---

## Context

Proiectul a început ca un magazin online. A evoluat într-un ecosistem care include:

- **Un blog educațional** (Knowledge Base) cu 80+ articole
- **Un marketplace** de anunțuri clasificate
- **Un magazin propriu-zis** cu coș, checkout și plăți Stripe
- **Endpoint-uri AI** pentru RAG, training data și chat
- **Un sistem de categorii ierarhice** cu 3 niveluri de adâncime

În loc să forțăm totul într-o singură aplicație, am ales să separăm în două binary-uri distincte care împărtășesc același set de module de bază (LEGO).

---

## 1. Problema: Monolit vs Separare

### Abordarea monolitică (greșită)

```
┌─────────────────────────────────────────────┐
│              Super App (monolit)             │
│                                              │
│  /biblioteca    → handler articole           │
│  /anunturi      → handler anunțuri           │
│  /cart          → handler coș                │
│  /checkout      → handler checkout + Stripe  │
│  /admin         → handler administrare       │
│  /api/ai/*      → handler AI/RAG             │
│                                              │
│  ❌ Un bug în checkout poate doborî și KB   │
│  ❌ O vulnerabilitate în auth afectează tot  │
│  ❌ Restart după deploy = downtime total     │
│  ❌ Dependințe grele (Stripe) și în KB       │
└─────────────────────────────────────────────┘
```

### Abordarea noastră: două aplicații, același LEGO

```
┌─────────────────────┐  ┌─────────────────────────┐
│    myapp (port 3000) │  │  shop-mvp (port 3001)   │
│                     │  │                         │
│  📚 Knowledge Base  │  │  🛒 Coș de cumpărături  │
│  🏪 Marketplace     │  │  💳 Checkout + Stripe   │
│  🤖 AI/RAG API      │  │  📦 Comenzi + istoric   │
│  📄 llms.txt / RSS  │  │  🔐 Admin panel         │
│  🗺️ Sitemap/robots  │  │  🚦 Rate limiting       │
│                     │  │  🛡️ Security headers    │
│  Fără Stripe        │  │  Fără AI, fără KB       │
│  Fără coș           │  │  Fără anunțuri          │
└─────────────────────┘  └─────────────────────────┘
        │                         │
        └──────────┬──────────────┘
                   │
        ┌──────────▼──────────────┐
        │   Module LEGO (libs/)   │
        │                         │
        │  rust-auth              │
        │  rust-cart              │
        │  rust-payment           │
        │  rust-slug              │
        │  rust-url-normalizer    │
        │  cache (PgCache)        │
        │  storage                │
        │  ... (15 crate-uri)     │
        └─────────────────────────┘
```

---

## 2. Arhitectura capability-based (seL4-style)

Inspirată de microkernel-ul seL4, fiecare handler primește DOAR capabilitățile de care are nevoie, nu întregul `AppState`.

### În `shop-mvp`:

```rust
// ❌ Greșit: handlerul poate accesa ORICE
async fn checkout_handler(State(state): State<AppState>) -> Response {
    state.payment.refund(...)?;  // de ce poate face refund un handler de checkout?
    state.auth.delete_user(...)?; // de ce poate șterge utilizatori?
}

// ✅ Corect: fiecare handler primește doar ce-i trebuie
async fn checkout_handler(State(state): State<OrderState>) -> Response {
    // Poate accesa DOAR: orders, cart, payment, auth (autentificare)
    // NU poate accesa: products, admin, settings
}
```

### Cum funcționează:

```rust
// state.rs — domain state-uri separate
#[derive(Clone)]
pub struct AuthState {
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}

#[derive(Clone)]
pub struct ProductState {
    pub products: Arc<dyn ProductRepo>,
    pub renderer: RenderService,
    pub db: PgPool,           // doar pentru categorii
}

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

// Sub-router-e cu state-uri diferite
let auth_routes = Router::new()
    .route("/login", post(login_handler))
    .with_state(auth_state);  // doar AuthState

let order_routes = Router::new()
    .route("/checkout", post(checkout_handler))
    .with_state(order_state);  // doar OrderState
```

### Beneficii:

| Aspect | Monolit | Capability-based |
|--------|---------|-----------------|
| **Verificabil la compilare** | ❌ Orice handler poate accesa orice | ✅ Imposibil din greșeală |
| **Testabilitate** | Trebuie să mock-uiesti tot AppState | ✅ Mock-uiești doar ce folosești |
| **Securitate** | Dacă un handler e spart, totul e spart | ✅ Daune limitate la domeniu |
| **Înțelegere** | Greu de urmărit dependințele | ✅ Explicit, citești direct din semnătură |

---

## 3. Module LEGO — Crate-uri reutilizabile

Fiecare modul e un crate separat în `libs/`, cu un **trait** (interfață) și o **implementare PostgreSQL** (Pg).

### Exemplu: `rust-payment`

```rust
// libs/rust-payment/src/lib.rs

/// Trait-ul — orice implementare de plată
#[async_trait]
pub trait PaymentRepo: Send + Sync {
    async fn create_payment(&self, amount: u32, currency: &str) -> Result<PaymentIntent, PaymentError>;
    async fn confirm_payment(&self, payment_id: &str) -> Result<PaymentStatus, PaymentError>;
    async fn refund(&self, payment_id: &str, amount: Option<u32>) -> Result<(), PaymentError>;
}

/// Implementare Stripe (cheamă API direct HTTP, nu SDK)
pub struct StripePayment {
    secret_key: String,
}

/// Error boundary: retry + timeout
pub struct RetryPayment<T: PaymentRepo> {
    inner: Arc<T>,
    max_retries: u32,
}
```

### Toate modulele:

| Crate | Trait | Implementare | Scop |
|-------|-------|-------------|------|
| `rust-auth` | `AuthRepo` | `PgAuthRepo` | JWT, login, signup |
| `rust-cart` | `CartRepo` | `PgCartRepo` | Coș de cumpărături |
| `rust-marketplace-products` | `ProductRepo` | `PgProductRepo` | Produse CRUD |
| `rust-marketplace-categories` | `CategoryRepo` | `PgCategoryRepo` | Categorii ierarhice |
| `rust-marketplace-orders` | `OrderRepo` | `PgOrderRepo` | Comenzi + items |
| `rust-marketplace-listings` | `ListingRepo` | `PgListingRepo` | Anunțuri clasificate |
| `rust-payment` | `PaymentRepo` | `StripePayment` | Plăți Stripe |
| `rust-slug` | — (funcții) | — | Slug-uri URL |
| `rust-url-normalizer` | — (middleware) | — | Normalizare URL |
| `rust-path-prefix` | — (extractor) | — | Detectare base path reverse proxy |
| `cache` | `Cache` | `PgCache` | Cache PostgreSQL cu TTL |
| `storage` | `Storage` | `FileStorage` | Stocare fișiere |
| `u32-i32-converter` | — (funcții) | — | Conversii sigure |

### Cum se asamblează:

```rust
// shop-mvp/src/main.rs — bootstrap LEGO

let products: Arc<dyn ProductRepo> = Arc::new(PgProductRepo::new(pool.clone(), Box::new(DummyCatSvc)));
let auth: Arc<dyn AuthRepo> = Arc::new(PgAuthRepo::new(pool.clone(), &jwt_secret));
let cart: Arc<dyn CartRepo> = Arc::new(PgCartRepo::new(pool.clone()));
let orders: Arc<dyn OrderRepo> = Arc::new(PgOrderRepo::new(pool.clone()));
let payment: Arc<dyn PaymentRepo> = Arc::new(RetryPayment::new(Arc::new(StripePayment::new(&stripe_secret))));
```

Fiecare linie e o piesă LEGO. Le poți înlocui individual fără să afectezi restul.

---

## 4. Knowledge Base + AI — Ce are `myapp` și nu are `shop-mvp`

### 4.1 Sistemul de articole

80+ articole educaționale în Markdown, scrise în limba română. Acoperă:

```
Fundamente: mașina Turing, lambda calculus, big O, P vs NP
Arhitectură: Von Neumann, Harvard, RISC-V, seL4, CHERI
Securitate: HSM, TEE, post-quantum crypto, Common Criteria
Matematică: algebra liniară, Bayes, entropie, numere prime
Sisteme: OS, TCP/IP, DNS, RTOS, post-posix
Practic: pgvector pe Android, mold+sccache, PRG pattern, zero-JS
```

Fiecare articol are:
- Conținut scris în Markdown cu `pulldown-cmark` → HTML
- Cuprins automat (ToC) din heading-uri
- Anchor links pentru navigare
- Tag-uri, dificultate, timp de citire
- Embedding pgvector pentru căutare semantică

### 4.2 Endpoint-uri AI/RAG

```bash
# Export training data (JSONL)
GET /api/ai/training/products
GET /api/ai/training/articles
GET /api/ai/training/all          # totul într-un JSON

# Retrieval (RAG)
GET /api/ai/retrieve/products?q=iphone
GET /api/ai/retrieve/articles/semantic?q=algoritmi   # pgvector

# Chat pe articol
POST /api/chat/{slug}
# Body: { "question": "Ce e big O?" }
# Răspuns: DeepSeek API + context din articol

# Discovery
GET /llms.txt          # sumar pentru LLM crawlers
GET /llms-full.txt     # index complet
GET /rss.xml           # RSS feed
GET /robots.txt        # permisiuni AI crawlers
GET /sitemap.xml       # sitemap dinamic
```

### 4.3 Căutarea semantică cu pgvector

```sql
-- Activare extensie
CREATE EXTENSION IF NOT EXISTS vector;

-- Tabela embeddings
CREATE TABLE article_embeddings (
    id SERIAL PRIMARY KEY,
    article_slug TEXT REFERENCES articles(slug),
    embedding vector(768),  -- nomic-embed-text
    chunk_text TEXT,
    chunk_index INT
);

-- Index IVFFlat pentru căutare rapidă
CREATE INDEX idx_embeddings_ivfflat
    ON article_embeddings
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

-- Căutare semantică: găsește articole similare
SELECT a.title, a.slug,
       ae.embedding <=> query_embedding AS distance
FROM articles a
JOIN article_embeddings ae ON a.slug = ae.article_slug
ORDER BY distance
LIMIT 10;
```

Embedding-urile sunt generate cu Ollama (model `nomic-embed-text`) direct pe mașina locală — zero costuri de API extern.

---

## 5. Filosofia din spate

### 5.1 Server-side first

Zero JavaScript în producție. Totul e server-side rendering cu form POST + 302 redirect (PRG pattern). Fiecare request produce HTML complet și final. Fără stare client-side, fără API calls după primul paint.

### 5.2 HN Philosophy

Textul e mai rapid decât JavaScript. O pagină din 2007 e mai robustă decât un SPA din 2026. Conținutul e text, nu aplicație. Cacheabil, accesibil, indexabil.

### 5.3 Respect pentru standarde

HTTP status codes corecți (302, 401, 403, 429). Cookie HttpOnly pentru JWT. CSP, X-Frame-Options, X-Content-Type-Options, Referrer-Policy. Fără hack-uri, fără soluții creative.

### 5.4 Capability security

Inspirat de microkernel-ul seL4 verificat formal. Fiecare componentă primește doar permisiunile de care are nevoie. Dacă un handler de coș e spart, atacatorul NU poate accesa plăți sau admin.

### 5.5 Dual-project separation

Separarea în două binary-uri (`myapp` + `shop-mvp`) oferă:
- **Izolare**: un crash în KB nu afectează magazinul
- **Deploy independent**: actualizezi articolele fără downtime la plăți
- **Securitate**: cheile Stripe doar în shop-mvp, nu în KB
- **Resource allocation**: scalezi magazinul independent de conținut

---

## 6. Ce urmează (Faza 4 — Enterprise)

- [x] Sitemap.xml + robots.txt (deja implementat în `myapp`)
- [ ] JSON-LD produse (date structurate)
- [ ] Factură/Recepisă PDF
- [ ] Anulare comandă de către user
- [ ] Backup DB automat
- [ ] Notificare email
- [ ] Wishlist
- [ ] Unificare myapp + shop-mvp (un singur deploy)

---

## Concluzie

Arhitectura dual-project cu module LEGO și capability-based design poate părea excesivă pentru un proiect mic. Dar beneficiile devin evidente imediat ce proiectul crește:

1. **Poți mock-ui orice** — fiecare modul are un trait, fiecare handler are un state mic
2. **Poți debug-ui orice** — request ID, SQL timing, panic hook, DB query counter
3. **Poți deploy-ui independent** — KB și magazinul au cicluri de viață separate
4. **Poți scala selectiv** — magazinul poate rula pe mai multe instanțe decât KB
5. **Poți înlocui orice modul** — Stripe cu BTCPay schimbând o singură linie

> *"Mai puțin înseamnă mai mult. Zero JS e mai stabil decât orice librărie. Două binary-uri sunt mai sigure decât un monolit."*

---

## Referințe

- [Arhitectura LEGO vs Hot Path](/biblioteca/arhitectura-lego-hotpath) — modelul hibrid
- [Filosofia HN în arhitectura web](/biblioteca/filosofia-hn-server-side) — de ce text > JS
- [Arhitectura Zero JavaScript](/biblioteca/arhitectura-zero-js) — călătoria de la HTMX la zero JS
- [PRG Pattern](/biblioteca/prg-pattern-impl) — Post-Redirect-Get în practică
- [seL4 — Verificare formală](/biblioteca/sel4) — microkernel-ul care inspiră arhitectura
