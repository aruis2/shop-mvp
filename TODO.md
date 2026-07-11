# TODO — Shop MVP

## Stare actuală (2026-07-11)
- **Teste:** 96/96 ✅
- **Warnings:** 0 ✅
- **Vulnerabilități:** 0 (cargo audit) ✅
- **InputFactory:** complet (parser.rs + 17 metode) ✅
- **OutputFactory:** complet (10 metode, integrat în toate handlerele) ✅
- **TRUST-BOUNDARY.md:** documentat ✅
- **CSP:** producție-ready ✅
- **Git:** inițializat, 12+ commituri ✅

---

## Nivel 1: Tehnic — cod (prioritate maximă)

### 1.1 Migrare handlere la InputFactory (parser.rs)
- [ ] Înlocuire `serde_urlencoded` cu `parse_form_into()` în toate handlerele
- [ ] `auth.rs` — login/signup form → `InputFactory::parse_email()` etc.
- [ ] `cart.rs` — add/remove form → `InputFactory::parse_slug()`, `parse_qty()`
- [ ] `orders.rs` — checkout form → `InputFactory::parse_name()`, `parse_address()`
- [ ] `admin.rs` — product create/update → `InputFactory::parse_brand()`, `parse_product_name()`
- [ ] Eliminare `serde_urlencoded` din Cargo.toml

### 1.2 Tipuri în modele (newtype pattern)
- [ ] Modelele din libs să folosească `Email`, `Price`, `UserId`, `OrderId` etc.
- [ ] Înlocuire `String` → `ShippingName`, `ShippingAddress` în OrderRepo
- [ ] Înlocuire `String` → `Slug` în ProductRepo
- [ ] Înlocuire `i32` → `ProductId`, `CategoryId`

### 1.3 Conformitate Rust cu standardele (din STANDARDS.md)

Rust ca limbaj acoperă nativ sau prin tooling standard următoarele cerințe din standardele noastre:

| Standard | Cerință | Cum o acoperă Rust | Status |
|---|---|---|---|
| **OWASP ASVS V2** (Authentication) | Password storage, session mgmt | JWT, hashing cu `rust-auth` | ✅ |
| **OWASP ASVS V5** (Validation) | Input validation | `InputFactory` + newtype pattern | ✅ |
| **OWASP ASVS V6** (Output Encoding) | XSS prevention | `OutputFactory::sanitize_context()` | ✅ |
| **OWASP ASVS V8** (Data Protection) | Cache control, CSP | Headere în `security_headers` | ✅ |
| **OWASP ASVS V9** (Communication) | TLS, HSTS | HSTS header, `upgrade-insecure-requests` | ✅ |
| **OWASP ASVS V10** (Malicious Code) | CSP, integrity | CSP strict (`script-src 'self'`) | ✅ |
| **OWASP API Top 10 #1** (Broken Object Level Auth) | Auth pe fiecare endpoint | Capability-based state, token per request | ✅ |
| **OWASP API Top 10 #7** (Security Misconfiguration) | Headere, CORS | `security_headers` middleware | ✅ |
| **CIS Control 7** (Vulnerability Mgmt) | Scan dependencies | `cargo audit` în CI | ✅ |
| **CIS Control 16** (App Security) | Secure coding | `cargo clippy -D warnings`, `#![deny(unsafe_code)]` | ✅ |
| **Rust Edition 2024** | Limbaj actualizat | `edition = "2024"` în Cargo.toml | ✅ |
| **Rust API Guidelines** | Conventions | Naming `snake_case`, trait-uri standard | ✅ |
| **Memory Safety** (vs C/C++) | Buffer overflow, UAF, double-free | **Nativ** — garantat de compilator | ✅ Gratis |
| **Type Safety** (vs TypeScript/JS) | Null, undefined, type coercion | **Nativ** — `Option<T>`, `Result<T,E>`, pattern matching | ✅ Gratis |
| **Thread Safety** (vs Java/Python) | Race conditions | **Nativ** — Send + Sync traits, ownership | ✅ Gratis |

#### Ce mai trebuie la nivel de limbaj/tooling

| Cerință | Standard | Instrument | Efort |
|---|---|---|---|
| Fuzzing pe InputFactory | OWASP ASVS V5 | `cargo fuzz` | ~3h |
| Code coverage | CIS Control 16 | `cargo tarpaulin` | ~30min |
| Supply chain audit | SLSA Level 1+ | `cargo vet` (Mozilla) | ~2h |
| Block known-vuln deps | OWASP API Top 10 #6 | `cargo deny` | ~30min |
| UB detection | ISO 26262 (ASIL) | MIRI | ~1h |
| Concurrency testing | OWASP ASVS V1 | `loom` | ~2h |
| SBOM generation | NTIA / SPDX | `cargo sbom` | ~30min |
| Bloat detection | — | `cargo bloat` | ~15min |
| Geiger counter (unsafe audit) | CIS Control 16 | `cargo geiger` | ~15min |
| PGO optimization | NIST SP 800-207 | `-C profile-generate` | ~2h

### 1.4 Securitate — IDOR (Insecure Direct Object Reference)

**IDOR** = Atacatorul modifică un ID (ex: `order_id=123` → `order_id=124`) și accesează datele altui utilizator.
OWASP API Top 10 #1 (Broken Object Level Authorization).

| Endpoint | Risc | Verificare proprietate? | Status |
|---|---|---|---|
| `GET /orders` | Userul A vede comenzile userului B | `get_orders_by_user(user.id)` — da, filtrat | ✅ |
| `POST /order/{id}/pay` | Userul A plătește comanda userului B | Verificare `order.user_id == user.id`? | ⚠️ **De verificat** |
| `POST /order/{id}/status` (admin) | Admin schimbă statusul oricărei comenzi | Doar admin — corect | ✅ |
| `GET /account/export` | Userul A exportă datele userului B | `verify_token()` + `delete_user(user.id)` | ✅ |
| `POST /cart/add` | Adaugi produse în coșul altcuiva | Session ID din cookie — corect | ✅ |
| `GET /admin/product/{slug}/edit` | Admin editează orice produs | Doar admin | ✅ |
| `DELETE /admin/product/{slug}` | Admin șterge orice produs | Doar admin | ✅ |

#### 🔍 IDOR — Insecure Direct Object Reference (OWASP API Top 10 #1)

**Ce este:** Atacatorul modifică un ID în request (ex: `GET /order/123` → `GET /order/124`) și accesează datele altui utilizator. ID-ul e perfect valid — problema e **lipsa verificării de proprietate**.

**Exemplu concret:** Dacă userul A trimite `POST /order/ord-{uuid}/pay` cu un `order_id` care nu îi aparține, plata merge în contul altcuiva.

**Cum se rezolvă:** În fiecare handler cu ID, verifici că obiectul aparține userului:
```rust
let order = s.orders.get_by_id(&order_id).await?;
if order.user_id != current_user.id {
    return Err("Nu poți accesa această comandă");
}
```

| Endpoint | Risc | Status |
|---|---|---|
| `GET /order/{id}/pay` | Userul A plătește comanda userului B | ⚠️ De verificat |
| `GET /orders` | Userul A vede comenzile userului B | ✅ Filtrat în query |
| `GET /admin/order/{id}/status` | Admin modifică comanda greșită | ⚠️ De verificat |
| `GET /admin/product/{slug}/edit` | Admin editează produs greșit | ✅ `slug` e în URL, vizibil |

- [ ] Audit: verifică fiecare endpoint cu `{id}` / `{slug}` că **proprietatea** e verificată
- [ ] `orders.rs` — în `order_pay()`, verifică `order.user_id == current_user.id`
- [ ] `orders.rs` — în `order_pay()`, verifică `payment_status != "paid"` (idempotency)
- [ ] `admin.rs` — în `admin_order_update_status()`, verifică că userul e admin
- [ ] Teste: încearcă să accesezi comanda altui user cu token diferit
- [ ] Teste: încearcă să plătești aceeași comandă de două ori
- [ ] Documentează modelul de autorizare per-endpoint în TRUST-BOUNDARY.md

#### 🏁 Race conditions (OWASP API Top 10 #4)

**Ce este:** Doi utilizatori cumpără simultan ultimul produs. Ambele requesturi văd `stock_count=1`, ambele trec validarea, ambele scad stocul → stoc final = -1.

**Cum se rezolvă:** PostgreSQL row-level locking (`SELECT ... FOR UPDATE`) în aceeași tranzacție.

- [ ] Audit: toate operațiile care citesc + scriu în același handler sînt în aceeași tranzacție?
- [ ] `cart.rs` — `add_item()`: tranzacție cu `FOR UPDATE` pe stoc?
- [ ] `orders.rs` — `checkout_handler()`: creează comanda, scade stocul, golește coșul — e atomic?
- [ ] Teste: simulare concurență (2 requesturi simultane pentru ultimul produs)
- [ ] Documentează modelul de concurență în PHILOSOPHY.md

#### 🎯 State machine (OWASP ASVS V2, V3)

**Ce este:** O comandă poate fi plătită doar o dată. Un refund poate fi cerut doar după plată. Un produs e livrat doar după expediere. Fără verificări, poți plăti de două ori sau returna un produs nelivrat.

**Cum se rezolvă:** Verificare explicită în fiecare handler:
```rust
if order.payment_status == "paid" {
    return Err("Comanda e deja plătită");
}
```

- [ ] Audit: toate tranzițiile de stare au verificări?
- [ ] `admin.rs` — previne tranziții invalide (ex: delivered → confirmed)
- [ ] `orders.rs` — previne plata dublă
- [ ] `orders.rs` — Stripe webhook: idempotency key
- [ ] Teste: încearcă toate tranzițiile invalide
- [ ] Documentează diagrama de stări în TRUST-BOUNDARY.md

### 1.6 🏭 LogicFactory — Uzina de logică business

**Concept:** Exact cum InputFactory parsează ORICE intrare și OutputFactory sanitarizează ORICE ieșire — **LogicFactory verifică ORICE regulă de business** într-un singur loc centralizat.

**Problema acum:** Verificările de autorizare, IDOR, state machine, stoc etc. sînt împrăștiate prin handlere. Unii handlere verifică, alții nu. E ușor de omis.

**Soluția:** Un singur punct de validare business, apelat de toate handlerele:

```rust
// Un singur loc unde definim TOATE regulile de business
pub struct LogicFactory;

impl LogicFactory {
    // ─── Autorizare ─────────────────────────────────
    pub fn verify_ownership<T: Eq>(user_id: &T, owner_id: &T) -> Result<(), LogicError> {
        (user_id == owner_id).then_some(()).ok_or(LogicError::Forbidden)
    }
    pub fn verify_admin(user: &CurrentUser) -> Result<(), LogicError> { ... }
    pub fn verify_role(user: &CurrentUser, required: &str) -> Result<(), LogicError> { ... }

    // ─── State machine ──────────────────────────────
    pub fn verify_not_paid(payment_status: &str) -> Result<(), LogicError> { ... }
    pub fn verify_status_transition(current: &str, next: &str) -> Result<(), LogicError> { ... }

    // ─── Business rules ─────────────────────────────
    pub fn verify_stock_available(stock: i32, qty: i32) -> Result<(), LogicError> { ... }
    pub fn verify_cart_not_empty(item_count: i64) -> Result<(), LogicError> { ... }
    pub fn verify_max_order_value(total_bani: i64, max: i64) -> Result<(), LogicError> { ... }
    pub fn verify_payment_amount(expected: i64, actual: i64) -> Result<(), LogicError> { ... }
}
```

**Avantaje:**
- **O singură modificare** cînd se schimbă o regulă — nu 7 handlere
- **Zero erori de omisiune** — toate handlerele trec prin aceleași verificări
- **Testabil izolat** — testezi `LogicFactory` o dată, nu în fiecare handler
- **Documentat** — citind `LogicFactory` știi TOATE regulile businessului

**Cum se integrează:**
```rust
// În fiecare handler, înainte de orice logică:
LogicFactory::verify_ownership(&current_user.id, &order.user_id)?;
LogicFactory::verify_not_paid(&order.payment_status)?;
LogicFactory::verify_stock_available(product.stock_count, req.qty)?;
```

- [ ] Creează `types/logic.rs` cu `LogicFactory` + `LogicError`
- [ ] Migrează verificările de autorizare din handlere în LogicFactory
- [ ] Migrează IDOR checks (ownership) în LogicFactory
- [ ] Migrează state machine checks în LogicFactory
- [ ] Adaugă teste unitare pentru fiecare metodă LogicFactory
- [ ] Adaugă teste de integrare: încearcă să încalci fiecare regulă
- [ ] Documentează în PHILOSOPHY.md: principiul #15 — LogicFactory

### 1.7 Securitate suplimentară (tooling)
- [ ] `cargo deny` — blocare dependințe cu vulnerabilități + licențe interzise
- [ ] `cargo tarpaulin` — code coverage (minim 80%)
- [ ] `cargo fuzz` — fuzzing pe InputFactory
- [ ] `cargo vet` — supply chain audit (Mozilla-style)
- [ ] MIRI — detectare undefined behavior
- [ ] `loom` — testare concurență
- [ ] Eliminare `#![allow(dead_code)]` — dead code check

### 1.4 Testare
- [ ] Teste integration cu Playwright (end-to-end)
- [ ] Teste pentru fiecare handler (nu doar template render)
- [ ] Teste de securitate specifice (XSS, open redirect, SQL injection)
- [ ] Teste de ratelimit și lockout

---

## Nivel 2: DevOps & CI/CD (prioritate medie)

### 2.1 CI/CD
- [ ] GitHub Actions: `cargo test`, `cargo clippy`, `cargo fmt`, `cargo audit`
- [ ] GitHub Actions: `cargo deny`, `cargo tarpaulin`
- [ ] GitHub Actions: security scan (trivy, etc.)
- [ ] GitHub Actions: Playwright e2e tests (cu PostgreSQL service)
- [ ] Deploy automat pe S22 (senmut.org) la push pe main

### 2.2 Infrastructură
- [ ] Docker multi-stage (deja există, verifică)
- [ ] Reverse proxy (Caddy/nginx) în fața Axum
- [ ] HTTPS (Let's Encrypt)
- [ ] Rate limiting la nivel de proxy (nu doar în app)
- [ ] WAF (Cloudflare sau similar)
- [ ] Monitoring (uptime, error tracking)
- [ ] Backup DB automat

### 2.3 Secrets management
- [ ] Verifică scriptul `secrets.sh` (age-based encryption)
- [ ] `.env.example` actualizat cu toate variabilele
- [ ] Rotire chei periodic

---

## Nivel 3: Standarde & Conformitate (prioritate variabilă)

### 3.1 Acoperire standarde (cod)
- [ ] OWASP ASVS L2 — verificare toate 250+ cerințe
- [ ] OWASP API Top 10 — verificare toate 10
- [ ] WCAG 2.1 AA — audit accesibilitate (contrast, keyboard nav, aria)
- [ ] PSD2/SCA — verificare flux plată (Stripe face SCA, noi redirecționăm)
- [ ] eIDAS — semnături electronice (necesită service extern)

### 3.2 Documentație standarde
- [ ] STANDARDS.md — actualizare cu roadmap 2030
- [ ] RACI matrix — responsabilități per standard
- [ ] Cost estimates per standard
- [ ] Dependency graph între standarde
- [ ] Gap analysis — ce lipșește față de fiecare standard

### 3.3 Conformitate legală (necesită avocat/auditor)
- [ ] GDPR — privacy policy (deja există pagină)
- [ ] GDPR — cookie consent
- [ ] GDPR — data export (deja există handler)
- [ ] GDPR — data deletion (deja există handler)
- [ ] PCI DSS — verificare că nu stocăm carduri (Stripe)
- [ ] ISO 27001 — SMS (procese, nu cod)
- [ ] CIS Controls — safeguards organizaționale

---

## Nivel 4: Arhitectură & Refactor (prioritate scăzută)

### 4.1 Performance
- [ ] Profiling cu `perf` / `flamegraph`
- [ ] Optimizare query-uri SQL (N+1, indexing)
- [ ] Connection pooling (sqlx deja face)
- [ ] Caching (redis sau in-memory)
- [ ] Paginare la toate listările (deja există parțial)

### 4.2 Refactor
- [ ] Separare teste în fișiere dedicate (nu în main.rs)
- [ ] Documentație rustdoc pe toate modulele publice
- [ ] Exemple în rustdoc
- [ ] Eliminare cod duplicat (parse_body apare în admin.rs și cart.rs)
- [ ] Uniformizare pattern-uri de error handling

### 4.3 Features
- [ ] Wishlist / favorite
- [ ] Review-uri produse
- [ ] Notificări (email/SMS pentru status comandă)
- [ ] Discount codes
- [ ] Variante produs (culoare, storage)
- [ ] Comparare produse
- [ ] Export comenzi (CSV/PDF)
- [ ] Temă dark
- [ ] i18n (EN/RO)

---

## Nivel 5: Hardware & OS (outsource/third-party)

- [ ] S22 (senmut.org) — build remote, DB host
- [ ] BIOS/UEFI — secure boot
- [ ] TPM — measured boot, remote attestation
- [ ] Kernel — hardened (kernel hardening configs)
- [ ] Network — firewall, IDS/IPS
- [ ] Physical security — acces la server
- [ ] Backup — off-site, tested restore

---

## Legendă priorități
- **🔴 Nivel 1** — Critic, blochează securitatea
- **🟡 Nivel 2** — Important, infrastructură
- **🟢 Nivel 3** — Standarde, conformitate
- **🔵 Nivel 4** — Îmbunătățiri
- **⚪ Nivel 5** — Hardware, out-of-scope MVP
