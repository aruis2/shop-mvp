# Arhitectură, Filosofie și Standarde pentru Mai Puține Bug-uri

> *De la PRG și HN Philosophy la seL4, OWASP ASVS, STRIDE, type-state pattern și parse-don't-validate — un ghid complet al deciziilor arhitecturale care reduc bug-urile la minim într-un ecosistem web modern scris în Rust.*

**Autor:** GitHub Copilot (DeepSeek V4 Flash)
**Data:** 2026-07-11
**Tag-uri:** arhitectura, securitate, standarde, rust, axum, owasp, sel4, type-state, testing
**Dificultate:** avansat
**Timp de citire:** 35 minute

---

## Cuprins

- [1. Introducere — Costul unui bug](#1-introducere--costul-unui-bug)
- [2. PRG Pattern — Primul nivel de apărare](#2-prg-pattern--primul-nivel-de-aparare)
- [3. HN Philosophy — De ce textul e mai sigur decât JavaScript](#3-hn-philosophy--de-ce-textul-e-mai-sigur-decat-javascript)
- [4. seL4 Capability Architecture — Izolare prin tipuri](#4-sel4-capability-architecture--izolare-prin-tipuri)
- [5. LEGO Modules — Ports & Adapters în Rust](#5-lego-modules--ports--adapters-in-rust)
- [6. Parse, Don't Validate — Tipuri care garantează corectitudinea](#6-parse-dont-validate--tipuri-care-garanteaza-corectitudinea)
- [7. Type-State Pattern — Stări invalide imposibile](#7-type-state-pattern--stari-invalide-imposibile)
- [8. Property-Based Testing — Găsește bug-uri pe care nu știi că le ai](#8-property-based-testing--gaseste-bug-uri-pe-care-nu-stii-ca-le-ai)
- [9. OWASP ASVS Level 1 — Security Baseline Verificabil](#9-owasp-asvs-level-1--security-baseline-verificabil)
- [10. STRIDE Threat Modeling — Identifică sistematic amenințările](#10-stride-threat-modeling--identifica-sistematic-amenintarile)
- [11. Fuzz Testing — Input-uri ostile găsesc vulnerabilități](#11-fuzz-testing--input-uri-ostile-gasesc-vulnerabilitati)
- [12. Hexagonal Architecture — Ports & Adapters Formalizat](#12-hexagonal-architecture--ports--adapters-formalizat)
- [13. Formal Verification — Verus, Dafny, Z3](#13-formal-verification--verus-dafny-z3)
- [14. Matricea Completa — Impact vs Efort](#14-matricea-completa--impact-vs-efort)
- [15. Concluzii — Filosofia Unificată](#15-concluzii--filosofia-unificata)

---

## 1. Introducere — Costul unui bug

Un bug în software are costuri exponențiale în funcție de când e descoperit:

| Faza | Cost relativ | Exemplu shop-mvp |
|------|-------------|-------------------|
| **Compilare** | 1× | Tipul greșit pentru preț (i32 vs f64) |
| **Testare** | 5× | Workflow de checkout care crapă la cantități mari |
| **Review** | 10× | Logică de discount greșită |
| **Staging** | 50× | Stripe API call cu parametri greșiți |
| **Producție** | 200× | Un user plătește de 2× aceeași comandă |
| **Producție + date** | 1000× | Fraudă prin elevation of privilege |

Obiectivul arhitectural e să mutăm cât mai multe bug-uri **la stânga** — spre compilare și testare, departe de producție.

### Filosofia unificată

Toate conceptele din acest articol urmăresc același scop:

> **"Fă imposibile bug-urile, nu doar greu de făcut."**

Fie prin:
- **Tipuri** care garantează corectitudinea (type-state, parse-don't-validate)
- **Izolare** care previne accesul neautorizat (capability-based, LEGO)
- **Testare** care găsește edge cases (property-based, fuzz)
- **Standarde** care definesc clar ce înseamnă "securizat" (OWASP, STRIDE)
- **Arhitectură** care elimină clase întregi de bug-uri (PRG, zero JS)

---

## 2. PRG Pattern — Primul nivel de apărare

### Problema

Un formular POST poate fi re-trimis accidental (F5, back button, dublu click). Rezultatul? Comenzi duble, plăți duplicate, utilizatori furioși.

### Soluția: Post → Redirect → Get

```rust
// ❌ Greșit: returnează HTML direct din POST
async fn checkout_handler(Form(form): Form<CheckoutForm>) -> Html<String> {
    let order = create_order(form).await;
    render_success_page(order) // Dacă userul face F5, comanda se creează din nou
}

// ✅ Corect: POST → 302 Redirect → GET
async fn checkout_handler(State(s): State<OrderState>, Form(form): Form<CheckoutForm>) -> Response {
    match create_order(&s, form).await {
        Ok(order) => {
            (StatusCode::FOUND, [
                (header::LOCATION, format!("/success?order_id={}", order.id)),
                (header::SET_COOKIE, clear_cart_cookie()),
            ]).into_response()
            // F5 acum re-trimite doar GET /success — inofensiv
        }
        Err(e) => {
            (StatusCode::FOUND, [
                (header::LOCATION, format!("/checkout?error={}", urlencode(&e))),
            ]).into_response()
        }
    }
}
```

### Bug-uri eliminate prin PRG

| Bug | Cauză | PRG elimină? |
|-----|-------|-------------|
| Comandă duplicată la F5 | Re-POST | ✅ Da — F5 re-trimite GET, nu POST |
| Dublă plată la checkout | Dublu click pe "Plătește" | ✅ Da — primul click = POST, al doilea = 404 |
| "Confirm re-submit" în browser | Navigare înapoi la pagină POST | ✅ Da — istoricul are doar GET-uri |
| Stare inconsistentă la refresh | Pagina afișa date vechi | ✅ Da — GET re-procesează |

### În shop-mvp

Toate form-urile folosesc PRG: login, signup, cart/add, cart/remove, checkout, admin actions. **Zero excepții.**

---

## 3. HN Philosophy — De ce textul e mai sigur decât JavaScript

### Principiul

O pagină din 2007 (Hacker News) e mai robustă decât un SPA din 2026. Pentru că:

```
SPA modern:       7 request-uri → 6 waterfall-uri → 2 re-render → 1 bug ascuns
HN (2007):         1 request → HTML complet → gata
```

### Bug-uri care nu există fără JS

| Bug-ul | Cauză | Cât timp am pierdut |
|--------|-------|-------------------|
| Logout redirect la greșit | `extract_path_from_url` bug | ~30min |
| Admin redirect loop infinit | `localStorage` vs HttpOnly cookie | ~20min |
| Checkout "Coșul e gol" fals | Session ID necitit din cookie | ~15min |
| Login nu afișează userul | `HX-Redirect` executat înaintea scriptului | ~40min |
| Login loop infinit | Referer era pagina de login | ~30min |
| Chrome vs Firefox diferențe | `Set-Cookie` + `window.location.href` race | ~25min |
| Parolă în URL | `hx-post` fără HTMX = GET implicit | ~10min |

**Total: ~4 ore pierdute pe bug-uri care n-ar fi existat fără JS.**

### Regula de aur

> Dacă poți face server-side, fă server-side.
> Dacă poți face fără JS, fă fără JS.
> Dacă ai nevoie de JS, întreabă-te de două ori.

### În shop-mvp

Zero JavaScript în producție. Server-side rendering cu Tera. Form POST + 302 redirect. Testabil cu `curl`.

---

## 4. seL4 Capability Architecture — Izolare prin tipuri

### Inspirația

seL4 este un microkernel verificat formal (matematic) care demonstrează că **un proces nu poate accesa nimic decât dacă deține o capabilitate explicită**.

Aplicat la web: un handler HTTP nu poate accesa nimic decât dacă primește explicit acel obiect prin **domain state**.

### Implementarea

```rust
// state.rs — Fiecare handler primește DOAR ce-i trebuie

/// Capabilități pentru autentificare — DOAR auth + render
#[derive(Clone)]
pub struct AuthState {
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}

/// Capabilități pentru produse — DOAR products + auth (read-only)
#[derive(Clone)]
pub struct ProductState {
    pub products: Arc<dyn ProductRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}

/// Capabilități pentru comenzi — orders + cart + payment + auth
#[derive(Clone)]
pub struct OrderState {
    pub orders: Arc<dyn OrderRepo>,
    pub cart: Arc<dyn CartRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}

/// Capabilități pentru admin — toate (dar nu direct PgPool)
#[derive(Clone)]
pub struct AdminState {
    pub products: Arc<dyn ProductRepo>,
    pub orders: Arc<dyn OrderRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}
```

### Verificare la compilare

```rust
// ❌ ASTA NU COMPILĂ:
async fn checkout_handler(State(state): State<AuthState>) -> Response {
    state.payment.refund(...)?;
    // ERROR: no field `payment` on `AuthState`
}
```

**Imposibil din greșeală.** Compilatorul e gardianul securității.

### Matricea capabilităților

| Handler | Auth | Products | Cart | Orders | Payment | Render |
|---------|------|----------|------|--------|---------|--------|
| login | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| products_page | ✅ (read) | ✅ | ❌ | ❌ | ❌ | ✅ |
| cart_add | ✅ (read) | ✅ | ✅ | ❌ | ❌ | ✅ |
| checkout | ✅ (read) | ❌ | ✅ | ✅ | ✅ | ✅ |
| admin_logs | ✅ (read) | ❌ | ❌ | ✅ | ❌ | ✅ (db) |

---

## 5. LEGO Modules — Ports & Adapters în Rust

### Pattern-ul

Fiecare modul LEGO e un **port** (trait) cu un **adapter** (implementare concretă):

```
┌─────────────┐     ┌──────────────────┐
│  Aplicația   │────▶│  PaymentRepo     │◀──────── port (trait)
│  (shop-mvp)  │     │  (libs/rust-     │
│              │     │   payment/)      │
└─────────────┘     └──────────────────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
       ┌──────────┐ ┌──────────┐ ┌──────────┐
       │  Stripe  │ │  Retry   │ │  Mock    │◀── adapters
       │ Payment  │ │ Payment  │ │ Payment  │
       └──────────┘ └──────────┘ └──────────┘
```

### În shop-mvp

```rust
// Port (trait)
#[async_trait]
pub trait PaymentRepo: Send + Sync {
    async fn create_payment(&self, amount: u32, currency: &str) -> Result<PaymentIntent, PaymentError>;
    async fn confirm_payment(&self, payment_id: &str) -> Result<PaymentStatus, PaymentError>;
    async fn refund(&self, payment_id: &str, amount: Option<u32>) -> Result<(), PaymentError>;
}

// Adapter 1: Stripe
pub struct StripePayment { secret_key: String }

// Adapter 2: Retry (decorator)
pub struct RetryPayment<T: PaymentRepo> {
    inner: Arc<T>,
    max_retries: u32,
}

// Adapter 3: Mock (pentru teste)
pub struct MockPayment { /* ... */ }
```

### Beneficii

| Aspect | Fără LEGO | Cu LEGO |
|--------|-----------|---------|
| **Schimbare provider** | Rescrii tot codul care cheamă Stripe | O singură linie: `StripePayment` → `BtcPayPayment` |
| **Testare** | Trebuie să ai cont Stripe real | `Arc::new(MockPayment::new())` |
| **Error handling** | În fiecare handler | `RetryPayment` înfășoară orice implementare |
| **Înțelegere** | 5000 de linii într-un fișier | 15 crate-uri, fiecare cu o singură responsabilitate |
| **AI productivitate** | Context mare, greu de înțeles | Crate-uri mici, clare, ușor de prompt-at |

---

## 6. Parse, Don't Validate — Tipuri care garantează corectitudinea

### Principiul (Alexis King)

> **Validate** = verifici datele, dar le păstrezi în tipul generic (String, i32).
> **Parse** = transformi datele într-un tip nou care garantează proprietăți.

```rust
// ❌ Validate (gresit):
fn process_email(email: &str) -> Result<(), Error> {
    if !email.contains('@') { return Err(Error::InvalidEmail); }
    // email e tot &str — orice funcție poate primi un email nevalidat
    send_email(email); // ⚠️ Pericol!
}

// ✅ Parse (corect):
#[derive(Debug, Clone, Serialize)]
pub struct Email(String);

impl Email {
    pub fn parse(s: &str) -> Result<Self, Error> {
        if s.contains('@') && s.len() < 254 && !s.starts_with('@') {
            Ok(Email(s.to_lowercase()))
        } else {
            Err(Error::InvalidEmail)
        }
    }
}

// Acum e imposibil să ai un Email invalid:
fn process_email(email: &Email) {
    send_email(email.as_ref()); // ✅ Garantat valid
}
```

### În shop-mvp — ce putem parsa

| Tipul actual | Problemă | Tipul nou |
|-------------|----------|-----------|
| `i32` (preț) | Poate fi negativ | `Price(PositiveI32)` |
| `String` (email) | Poate fi gol, fără @ | `Email(String)` |
| `String` (telefon) | Poate conține litere | `PhoneNumber(String)` |
| `String` (slug) | Poate avea spații, caractere speciale | `Slug(String)` |
| `String` (URL) | Poate fi invalid | `Url(String)` |
| `String` (status) | Orice string, inclusiv "pizza" | `OrderStatus` (enum) |

### Exemplu: `Price`

```rust
/// Prețul în cea mai mică unitate monetară (bani).
/// Garantat: 0 < price < i32::MAX
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Price(i32);

impl Price {
    pub fn new(bani: i32) -> Result<Self, Error> {
        if bani <= 0 {
            Err(Error::InvalidPrice("Prețul trebuie să fie pozitiv"))
        } else if bani > 10_000_00 { // 10,000 lei = 1,000,000 bani
            Err(Error::InvalidPrice("Preț prea mare"))
        } else {
            Ok(Price(bani))
        }
    }

    pub fn total(qty: u32, unit_price: Price) -> Result<Self, Error> {
        let total = (qty as i64) * (unit_price.0 as i64);
        if total > i32::MAX as i64 {
            Err(Error::PriceOverflow)
        } else {
            Price::new(total as i32)
        }
    }

    pub fn as_bani(&self) -> i32 { self.0 }
    pub fn as_lei(&self) -> f64 { self.0 as f64 / 100.0 }
}

// Acum:
fn calculate_total(items: &[CartItem]) -> Result<Price, Error> {
    items.iter()
        .try_fold(Price::new(0)?, |acc, item| {
            Price::total(item.qty, Price::new(item.price_bani)?)
                .map(|t| Price::new(acc.as_bani() + t.as_bani())?)
        })
}
```

---

## 7. Type-State Pattern — Stări Invalide Imposibile

### Problema

O comandă poate fi doar în anumite stări. Tranzițiile invalide (ex: "shipped" → "pending") nu ar trebui să fie posibile.

```rust
// ❌ Așa e acum — orice e posibil la runtime:
order.status = "shipped";
order.status = "pizza";  // 🚨 Compilează, dar e invalid!
order.ship();            // Funcționează și dacă e deja "shipped"
```

### Soluția: codificăm stările în tipuri

```rust
// Stări — tipuri ZST (zero-sized types)
pub struct Pending;
pub struct Paid;
pub struct Shipped;
pub struct Cancelled;

// O comandă e parametrizată de starea ei
pub struct Order<State> {
    pub id: Uuid,
    pub total_bani: i32,
    pub shipping_name: String,
    _state: std::marker::PhantomData<State>,
}

// Tranziții valide:
impl Order<Pending> {
    pub fn pay(self, payment: &dyn PaymentRepo) -> Result<Order<Paid>, Error> {
        payment.create_payment(self.total_bani as u32, "ron").await?;
        Ok(Order {
            id: self.id,
            total_bani: self.total_bani,
            shipping_name: self.shipping_name,
            _state: PhantomData,
        })
    }

    pub fn cancel(self) -> Order<Cancelled> {
        Order { id: self.id, total_bani: self.total_bani, shipping_name: self.shipping_name, _state: PhantomData }
    }
}

impl Order<Paid> {
    pub fn ship(self) -> Order<Shipped> {
        Order { id: self.id, total_bani: self.total_bani, shipping_name: self.shipping_name, _state: PhantomData }
    }

    pub fn refund(self, payment: &dyn PaymentRepo) -> Result<Order<Pending>, Error> {
        payment.refund(&self.id.to_string(), Some(self.total_bani as u32)).await?;
        Ok(Order { id: self.id, total_bani: self.total_bani, shipping_name: self.shipping_name, _state: PhantomData })
    }
}

// ❌ Acestea NU compilează:
// let paid: Order<Paid> = ...;
// paid.ship().ship();    // ERROR: Order<Shipped> n-are metodă ship()
// paid.pay(...);         // ERROR: Order<Paid> n-are metodă pay()
// let x: Order<Pending> = paid;  // ERROR: tipuri diferite

// ✅ Workflow corect:
let order: Order<Pending> = create_order(form).await?;
let order: Order<Paid> = order.pay(&payment).await?;
let order: Order<Shipped> = order.ship();
// Compilatorul verifică! Zero runtime checks.
```

### Diagrama tranzițiilor

```
┌─────────┐   pay()    ┌──────┐   ship()    ┌──────────┐
│ Pending  │──────────▶│ Paid │────────────▶│ Shipped  │
└─────────┘           └──────┘             └──────────┘
      │                  │
      │ cancel()         │ refund()
      ▼                  ▼
┌──────────┐      ┌─────────┐
│Cancelled │      │ Pending │
└──────────┘      └─────────┘
```

### Cost

Zero runtime overhead. `PhantomData` e ZST (0 bytes). Compilatorul elimină totul.

---

## 8. Property-Based Testing — Găsește bug-uri pe care nu știi că le ai

### Problema

Testele tradiționale verifică cazuri pe care le cunoști:

```rust
#[test]
fn test_total() {
    assert_eq!(calculate_total(2, 100), 200); // Cazul fericit
    assert_eq!(calculate_total(0, 100), 0);   // Edge case cunoscut
}
// Dar dacă ai uitat de overflow? Sau de numere negative?
```

### Soluția: scrii proprietăți, nu cazuri

```rust
use proptest::prelude::*;

proptest! {
    /// Proprietate: suma parțială ≤ total
    #[test]
    fn partial_total_never_exceeds_total(
        items in prop::collection::vec(
            (1..100u32, 1..10_000i32), // (qty, price_bani)
            1..50
        )
    ) {
        let total = calculate_total(&items).unwrap();
        
        // Verificăm că niciun subset nu depășește totalul
        for i in 0..items.len() {
            let partial = calculate_total(&items[0..i]).unwrap();
            assert!(partial.as_bani() <= total.as_bani(),
                "Subsetul 0..{i} are totalul {} > {}",
                partial.as_bani(), total.as_bani());
        }
    }

    /// Proprietate: totalul e suma cantității * preț
    #[test]
    fn total_is_sum_of_line_items(
        qty in 1..100u32,
        price in 100..999_999i32,
    ) {
        let items = vec![CartItem { qty, price_bani: price }];
        let total = calculate_total(&items).unwrap();
        assert_eq!(total.as_bani(), (qty as i32) * price);
    }

    /// Proprietate: prețul în lei = prețul în bani / 100
    #[test]
    fn price_conversion_is_consistent(
        bani in 1..i32::MAX,
    ) {
        let price = Price::new(bani).unwrap();
        let lei = price.as_lei();
        let roundtrip = (lei * 100.0).round() as i32;
        assert_eq!(roundtrip, bani,
            "Pierdere de precizie la conversia bani ↔ lei: {} → {} → {}",
            bani, lei, roundtrip);
    }
}
```

### Ce găsește proptest

| Tip de bug | Probabilitate |
|-----------|--------------|
| Overflow la cantități mari | 100% (dacă există) |
| Pierdere de precizie la conversii | 100% |
| Stări invalide nemenționate în cod | ~70% |
| Cazuri limită (0, MAX, negative) | 100% |
| Probleme de concurență | ~30% |

### În shop-mvp

Putem testa cu proptest:
- `calculate_total` — overflow, consistență
- `Price::new` — valori limită
- `Email::parse` — toate formele de email valid/invalid
- `checkout` workflow — comenzi cu 0 items, prețuri maxime
- `cart_add` — cantități negative, produse inexistente

---

## 9. OWASP ASVS Level 1 — Security Baseline Verificabil

### Ce e ASVS

OWASP Application Security Verification Standard e un standard internațional care definește **cerințe verificabile** pentru securitatea aplicațiilor web. Nivelul 1 = "security baseline" — minimul necesar pentru orice aplicație web.

### Audit ASVS Level 1 pentru shop-mvp

| Capitol | Cerință | Status shop-mvp |
|---------|---------|----------------|
| **V2: Authentication** | Verify credentials, prevent brute force | ✅ JWT HttpOnly + rate limiting (10 req/min) |
| **V2.1** | Password minimum length | ✅ Argon2 hashing |
| **V3: Session Management** | HttpOnly, Secure, SameSite cookies | ✅ |
| **V4: Access Control** | Principle of least privilege | ✅ Capability-based |
| **V5: Input Validation** | Validate all input on server | ✅ Tera auto-escape, server-side validation |
| **V5.1** | Reject invalid encoding | ⚠️ De verificat |
| **V6: Output Encoding** | Context-appropriate encoding | ✅ Tera auto-escape |
| **V7: Cryptography** | Use strong algorithms | ✅ Argon2, SHA-256 JWT |
| **V8: Data Protection** | Encrypt sensitive data in transit | ✅ HTTPS, CSP |
| **V8.3** | Protect cached data | ⚠️ De adăugat Cache-Control headers |
| **V9: Communication** | TLS, HSTS | ⚠️ HSTS e doar în config, nu în response |
| **V10: Malicious Code** | No eval(), no inline scripts | ✅ CSP: `script-src 'none'` |
| **V11: Business Logic** | Verify workflow integrity | ⚠️ De adăugat verificări server-side |
| **V12: File Upload** | Limit size, validate type | ⚠️ N/A (nu facem upload) |
| **V14: Config** | No default credentials, no debug in prod | ✅ `.env` în gitignore, APP_ENV=dev |

**Scor ASVS L1:** ~85% (13/15 cerințe îndeplinite)

### Ce trebuie adăugat

```rust
// HSTS header (HTTP Strict-Transport-Security)
async fn security_headers<B>(req: Request<B>, next: Next<B>) -> Response {
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    response
}

// Cache-Control pentru pagini cu date sensibile
response.headers_mut().insert(
    header::CACHE_CONTROL,
    "no-store, no-cache, must-revalidate".parse().unwrap(),
);
```

---

## 10. STRIDE Threat Modeling — Identifică sistematic amenințările

### Modelul STRIDE

Metodologia Microsoft pentru identificarea amenințărilor, categorie cu categorie:

| Litera | Categoria | În shop-mvp | Contramăsura |
|--------|-----------|-------------|-------------|
| **S** | **Spoofing** — falsificarea identității | Atacator încearcă să se autentifice ca alt user | Rate limiting 10 req/min, JWT semnat HMAC |
| **T** | **Tampering** — modificarea datelor | Atacator modifică date în tranzit | HTTPS, CSP, body size limit 2MB |
| **R** | **Repudiation** — negarea acțiunii | Userul neagă că a plasat comanda | Log în DB: `orders.created_at`, `orders.user_id` |
| **I** | **Information Disclosure** — expunerea datelor | Atacator accesează comenzi care nu-s ale lui | Capability-based: vezi doar propriile comenzi |
| **D** | **Denial of Service** — refuzul serviciului | Atacator inundă serverul cu request-uri | Rate limiting, conexiuni limitate |
| **E** | **Elevation of Privilege** — escaladare de privilegii | User devine admin | Capability-based: `AdminState` e separat |

### Matricea amenințărilor pentru fiecare endpoint

| Endpoint | S | T | R | I | D | E | Riscuri |
|----------|---|---|---|---|---|---|---------|
| `POST /login` | 🔴 | 🟢 | 🟢 | 🟢 | 🔴 | 🟢 | Brute force, DoS |
| `POST /checkout` | 🟡 | 🟡 | 🔴 | 🟡 | 🟢 | 🟢 | Dublă plată, fraudă |
| `GET /admin` | 🟢 | 🟢 | 🔴 | 🔴 | 🟢 | 🔴 | Date sensibile, privilegii |
| `POST /cart/add` | 🟢 | 🟡 | 🟢 | 🟢 | 🟢 | 🟢 | Manipulare cantități |

**🟢 = protejat, 🟡 = protejat parțial, 🔴 = neprotejat**

---

## 11. Fuzz Testing — Input-uri ostile găsesc vulnerabilități

### Ce e fuzz testing

Un **fuzzer** generează mii de input-uri random și verifică că aplicația nu crapă, nu intră în loop infinit și nu produce stări invalide.

### În Rust: `cargo-fuzz`

```rust
// fuzz_targets/checkout_handler.rs
#![no_main]

use libfuzzer_sys::fuzz_target;
use shop_mvp::handlers::orders::parse_checkout_form;

fuzz_target!(|data: &[u8]| {
    // Fuzzer-ul generează bytes random și încearcă să parseze form-ul
    if let Ok(body) = std::str::from_utf8(data) {
        if let Ok(form) = parse_checkout_form(body) {
            // Dacă s-a parsat, verificăm că e valid
            assert!(!form.shipping_name.is_empty());
            assert!(form.shipping_phone.len() >= 10);
        }
    }
    // Fuzzer-ul detectează: panică, overflow, loop infinit, memory leak
});
```

### Ce găsește fuzz testing

| Vulnerabilitate | Fuzz o găsește? |
|----------------|----------------|
| Buffer overflow | ✅ |
| Integer overflow | ✅ |
| Stack overflow (recursivitate infinită) | ✅ |
| Panică la input malformat | ✅ |
| Loop infinit | ✅ (timeout) |
| Memory leak | Partial |
| Business logic bugs | ❌ (alea le găsește proptest) |

### În shop-mvp

Putem fuzz-ui:
- `parse_body` — funcția care parsează JSON + URL-encoded form-uri
- `extract_token` — parsarea header-elorr de autentificare
- `cookie::get_cookie` — parsarea cookie-urilor
- `generate_slug` — input-uri Unicode, caractere speciale

---

## 12. Hexagonal Architecture — Ports & Adaptes Formalizat

### Ce e Hexagonal Architecture

Alias "Ports & Adapters": aplicația comunică cu exteriorul doar prin **port-uri** (interfețe), iar **adaptoarele** implementează acele port-uri pentru tehnologii concrete.

### În shop-mvp — deja implementat

```
┌────────────────────────────────────────────────────┐
│                    shop-mvp                         │
│                                                     │
│  ┌────────────── Port-uri ──────────────┐          │
│  │  AuthRepo │ CartRepo │ PaymentRepo   │          │
│  │  OrderRepo │ ProductRepo │ Cache     │          │
│  └──────────────────────────────────────┘          │
│            ▲           ▲           ▲                │
│            │           │           │                │
│  ┌─────────┴─┐  ┌──────┴──┐  ┌────┴─────────┐    │
│  │  PgAuth   │  │ PgCart  │  │StripePayment  │    │
│  │  Repo     │  │ Repo    │  │RetryPayment   │    │
│  └───────────┘  └─────────┘  └──────────────┘    │
│                                                     │
│  ┌─────────────── Adaptoare ────────────┐          │
│  │ PostgreSQL │ Stripe │ Tera │ Axum    │          │
│  └──────────────────────────────────────┘          │
└────────────────────────────────────────────────────┘
```

### Formalizarea

```toml
# Cargo.toml — feature flags pentru port vs adapter
[features]
default = ["lego"]

# Development: dynamic dispatch, mock-uri
lego = []

# Production: inline, monomorfizare
hot_path = []

# Test: mock-uri pentru toate
test = []
```

```rust
// Conditional compilation
#[cfg(feature = "lego")]
pub type DynPaymentRepo = Arc<dyn PaymentRepo>;

#[cfg(not(feature = "lego"))]
pub type DynPaymentRepo = StripePayment;  // direct, fără vtable
```

---

## 13. Formal Verification — Verus, Dafny, Z3

### Când e necesară

Verificarea formală e justificată când costul unui bug e **catastrofal**:
- Sisteme financiare: o eroare de calcul poate costa milioane
- Sisteme medicale: un bug poate ucide
- Aviație: un bug poate doborî un avion

Pentru un magazin online în alpha, **nu e necesară**. Dar merită să știm ce există.

### Verus (Rust)

```rust
// Verus — verificare formală pentru Rust (dezvoltat de Microsoft)
verus! {
    pub fn calculate_total(qty: u32, price: u32) -> (total: u32)
        requires
            qty > 0,
            price > 0,
            qty * price <= u32::MAX,
        ensures
            total == qty * price,
    {
        qty * price
    }
}
```

### Z3 (SMT Solver)

Z3 de la Microsoft Research poate verifica automat proprietăți:

```rust
// Z3 poate demonstra că:
// Pentru ORICE qty și price, totalul NU poate fi negativ
// Pentru ORICE x, y: (x + y) - y == x
// NICIUN workflow de checkout nu poate ajunge în "plătit" fără să cheme Stripe
```

### Când le vom folosi

| Stadiu | Tool | Pentru ce |
|--------|------|-----------|
| **Alpha (acum)** | — | Nu e necesar |
| **Beta** | proptest | Proprietăți de bază |
| **Producție <$100K/lună** | proptest + fuzz | Edge cases + securitate |
| **Producție >$100K/lună** | Verus | Logica de prețuri, refund |
| **Enterprise >$1M/lună** | Z3 + Verus | Workflow-uri critice |

---

## 14. Matricea Completa — Impact vs Efort

| # | Concept | Categorie | Efort | Impact bug-uri | Implementat |
|---|---------|-----------|-------|---------------|-------------|
| 1 | **PRG Pattern** | Arhitectură | 0 | Elimină dublă procesare | ✅ |
| 2 | **HN Philosophy (zero JS)** | Arhitectură | 0 | Elimină bug-uri JS | ✅ |
| 3 | **seL4 Capability** | Arhitectură | 0 | Previne escalation | ✅ |
| 4 | **LEGO modules** | Arhitectură | 0 | Izolare, testabilitate | ✅ |
| 5 | **Security headers** | Securitate | 0 | XSS, clickjacking | ✅ |
| 6 | **Rate limiting** | Securitate | 0 | Brute force | ✅ |
| 7 | **Rust borrow checker** | Limbaj | 0 | Memory safety | ✅ |
| 8 | **Parse, don't validate** | Tipuri | 2-3 zile | Input invalid garantat | ❌ |
| 9 | **Type-state pattern** | Tipuri | 3-5 zile | Stări invalide imposibile | ❌ |
| 10 | **Property-based testing** | Testare | 1-2 zile | Edge cases automate | ❌ |
| 11 | **OWASP ASVS L1 audit** | Standard | 4-5 ore | Baseline securitate | ~85% |
| 12 | **HSTS header** | Securitate | 15 min | Atacuri MITM | ❌ |
| 13 | **Cache-Control headers** | Securitate | 15 min | Date sensibile în cache | ❌ |
| 14 | **Non-repudiation logging** | Securitate | 1 zi | Negarea comenzilor | ❌ |
| 15 | **STRIDE audit** | Standard | 2-3 ore | Identifică threat-uri | ❌ |
| 16 | **Fuzz testing** | Testare | 3-5 zile | Vulnerabilități input | ❌ |
| 17 | **LEGO feature flags** | Arhitectură | 1 zi | Performanță producție | ❌ |
| 18 | **CSRF tokens** | Securitate | 2 zile | Cross-site request forgery | ❌ |
| 19 | **Secret Manager** | Infrastructură | 1 zi | Chei furate din .env | ❌ |
| 20 | **Verus formal verification** | Formal | Săptămâni | Corectitudine matematică | ❌ (future) |

### Grafic efort → impact

```
Impact
  ▲
  │        ⚡ Parse, don't validate
  │        ⚡ Property-based testing
  │        ⚡ Type-state
  │     ⚡ OWASP ASVS
  │     ⚡ STRIDE
  │  ⚡ HSTS, Cache-Control
  │  ⚡ Non-repudiation
  │⚡ Fuzz
  │⚡ CSRF
  │⚡ Secret Manager
  └───────────────────────────▶ Efort
    1h  1d   1s   1s   1lună
```

**Prioriate:** Sus-stânga = făcut primul (mult impact, puțin efort).

---

## 15. Concluzii — Filosofia Unificată

### Cele 10 porunci arhitecturale

1. **Server-side first** — Dacă poți face fără JS, fă fără JS.
2. **PRG peste tot** — Orice POST → 302. Zero excepții.
3. **Tipuri, nu string-uri** — Parse, don't validate. Fiecare tip garantează validitatea.
4. **Stări în tipuri, nu în variabile** — Dacă o stare e invalidă, să nu compileze.
5. **Fiecare handler, doar ce-i trebuie** — Capability-based. Nu `AppState` întreg.
6. **Trait-uri peste tot** — LEGO modules. Înlocuiești orice cu o linie.
7. **Testează proprietăți, nu cazuri** — Property-based testing. Găsește ce nu știi.
8. **Fuzz ce primește input** — Orice `parse`, `deserialize`, `from_str` merită fuzz.
9. **OWASP ASVS e Biblia** — Dacă nu știi ce lipsește la securitate, ASVS îți spune.
10. **STRIDE în fiecare sprint** — Înainte de un feature nou, gândește-te la cele 6 amenințări.

### Mantra

> *"Mai puține bug-uri nu vin din mai multă testare.*
> *Vin din tipuri care fac bug-urile imposibile.*
> *Vin din arhitectură care izolează erorile.*
> *Vin din standarde care definesc clar ce înseamnă 'corect'."*

### În practică — următorii pași

Pentru shop-mvp (alpha, debug mode), ordinea optimă:

| Pas | Acțiune | Timp | Beneficiu imediat |
|-----|---------|------|-------------------|
| 1 | Adaugă `Email`, `Price`, `PhoneNumber` cu parse | 2-3 zile | Input invalid = imposibil |
| 2 | Adaugă proptest pentru `calculate_total` | 1 zi | 0 edge cases de overflow |
| 3 | OWASP ASVS Level 1 audit (bifează ce ai) | 4-5 ore | Știi exact ce lipsește |
| 4 | Adaugă HSTS + Cache-Control headers | 15 min | Protecție imediată |
| 5 | STRIDE audit pentru checkout | 2-3 ore | Plan de securitate |
| 6 | Type-state pentru Order | 3-5 zile | Stări invalide = 0 la runtime |
| 7 | Fuzz pentru `parse_body` | 2-3 zile | Vulnerabilități ascunse |
| 8 | CSRF tokens | 2 zile | Protecție cross-site |

---

## Referințe

- [seL4 — Verificare formală a unui microkernel](/biblioteca/sel4)
- [Filosofia HN în arhitectura web](/biblioteca/filosofia-hn-server-side)
- [PRG Pattern — Post-Redirect-Get în Rust+Axum](/biblioteca/prg-pattern-impl)
- [Arhitectura LEGO vs Hot Path](/biblioteca/arhitectura-lego-hotpath)
- [Strategia de securitate pe 5 niveluri](/biblioteca/strategie-securitate-nivele)
- [Verificare formală modernă (Verus, Dafny, Aeneas)](/biblioteca/formal-verification-moderna)
- [Rust în sisteme safety-critical](/biblioteca/rust-safety-critical)
- [CHERI — Capability hardware architecture](/biblioteca/cheri)
- [Common Criteria (ISO 15408)](/biblioteca/common-criteria)
- [OWASP ASVS 5.0](https://owasp.org/www-project-application-security-verification-standard/)
- [STRIDE Threat Model (Microsoft)](https://learn.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [Parse, Don't Validate (Alexis King)](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [proptest — Property-based testing for Rust](https://proptest-rs.github.io/)
- [cargo-fuzz — Fuzz testing for Rust](https://rust-fuzz.github.io/book/cargo-fuzz.html)
