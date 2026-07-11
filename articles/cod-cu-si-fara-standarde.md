# 🆚 Fără standards vs Cu standards — Cod comparativ

> Exemple reale din shop-mvp: ce am fi scris fără standards și ce am scris CU standards.

---

## 1. Email — fără vs cu Parse Don't Validate

### ❌ Fără standards (String peste tot)

```rust
// handlers/auth.rs
async fn signup_handler(body: String) -> Response {
    let email = extract_field(&body, "email").unwrap_or("");
    // email e String — poate fi ORICE
    // "". "fără@". "a@b". "@@@". null bytes. XSS in email?
    
    if !email.contains('@') {
        return error_page("Email invalid");
    }
    // ⚠️ Validarea e doar aici — dacă altcineva cheamă create_user
    // cu un email nevalidat? Nimic nu-l oprește.
    create_user(email, password).await
}

// create_user.rs — 500 de linii mai departe
fn create_user(email: &str, password: &str) {
    // Nu mai verifică emailul — "sigur e valid, a trecut de signup"
    // ❌ Bug: dacă un admin adaugă manual un user?
    // ❌ Bug: dacă un import script uită să valideze?
}
```

**Bug-uri posibile:** email gol, SQL injection prin email, XSS, confuzie cu alt cîmp.

### ✅ Cu standards (newtype + parse)

```rust
// types/email.rs
#[derive(Debug, Clone, Serialize)]
pub struct Email(String);

impl Email {
    /// SINGURA modalitate de a crea un Email.
    /// Orice Email e GARANTAT valid.
    pub fn parse(s: &str) -> Result<Self, EmailError> {
        let s = s.trim().to_lowercase();
        if s.is_empty() { return Err(EmailError::Empty); }
        if !s.contains('@') { return Err(EmailError::MissingAt); }
        if s.starts_with('@') || s.ends_with('@') { return Err(EmailError::InvalidFormat); }
        if s.len() > 254 { return Err(EmailError::TooLong); }
        // ✅ Doar litere, cifre, @, ., -, _
        if !s.chars().all(|c| c.is_alphanumeric() || "@._-".contains(c)) {
            return Err(EmailError::InvalidChars);
        }
        Ok(Email(s))
    }
    
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn domain(&self) -> &str { self.0.split('@').nth(1).unwrap_or("") }
    pub fn local(&self) -> &str { self.0.split('@').next().unwrap_or("") }
}

// handlers/auth.rs
async fn signup_handler(body: String) -> Response {
    let email_str = extract_field(&body, "email").unwrap_or("");
    
    // O SINGURĂ validare — aici, la parsare
    let email = match Email::parse(email_str) {
        Ok(e) => e,
        Err(_) => return error_page("Email invalid"),
    };
    // email e Email — garantat valid, nu mai verifici niciodată
    
    create_user(&email, password).await
    // create_user cere &Email — nu poți pasa un string greșit
}

fn create_user(email: &Email, password: &str) {
    // ✅ Email e garantat valid — zero verificări
    db.execute("INSERT INTO users (email) VALUES ($1)", &[email.as_str()]);
}
```

**Bug-uri eliminate:** email gol, format invalid, caractere interzise, confuzie cu String.

---

## 2. Status comandă — fără vs cu Type-State

### ❌ Fără standards (String)

```rust
pub struct Order {
    pub status: String, // "pending" | "paid" | "shipped" | "pizza" ???
}

impl Order {
    pub fn pay(&mut self) {
        if self.status != "pending" {
            panic!("Comanda nu e în starea corectă!");
        }
        // ⚠️ Verificare la RUNTIME — testele trebuie să prindă asta
        self.status = "paid".to_string();
    }
    
    pub fn ship(&mut self) {
        if self.status != "paid" {
            panic!("Nu poți expedia o comandă neplătită!");
        }
        self.status = "shipped".to_string();
    }
}

// Test care trece:
#[test]
fn test_order_flow() {
    let mut o = Order { status: "pending".into() };
    o.pay();
    o.ship();
}

// Test care crapă la RUNTIME:
#[test]
fn test_ship_without_pay() {
    let mut o = Order { status: "pending".into() };
    o.ship(); // 💥 panic! — dar compilatorul n-a zis nimic
}

// ❌ Nici măcar nu e nevoie de test:
fn main() {
    let mut o = Order { status: "pizza".into() };
    o.pay(); // 💥 panic! — status invalid, dar compilatorul nu știe
}
```

**Bug-uri:** statusuri invalide, încălcare flow (ship fără pay), dublă expediere, panică la runtime.

### ✅ Cu standards (Type-State)

```rust
// Stări — tipuri ZST (zero bytes, zero cost)
pub struct Pending;
pub struct Paid;
pub struct Shipped;

pub struct Order<State> {
    pub id: Uuid,
    pub total_bani: i32,
    _state: PhantomData<State>, // 0 bytes
}

impl Order<Pending> {
    /// ✅ Doar Pending poate fi plătit — garantat la COMPILARE
    pub async fn pay(self, payment: &dyn PaymentRepo) -> Result<Order<Paid>, Error> {
        payment.create(self.total_bani).await?;
        Ok(Order { id: self.id, total_bani: self.total_bani, _state: PhantomData })
    }
}

impl Order<Paid> {
    /// ✅ Doar Paid poate fi expediat — garantat la COMPILARE
    pub fn ship(self) -> Order<Shipped> {
        Order { id: self.id, total_bani: self.total_bani, _state: PhantomData }
    }
}

// ❌ Niciunul dintre astea NU COMPILEAZĂ:
// let o = Order::<Pending> { ... };
// o.ship();               // ERROR: Order<Pending> n-are metodă ship()
// o.pay()?.ship().ship(); // ERROR: Order<Shipped> n-are ship()
// o.pay()?.pay();         // ERROR: Order<Paid> n-are pay()

// ✅ Singurul flow corect — garantat de compilator:
let o: Order<Pending> = create_order(form).await?;
let o: Order<Paid> = o.pay(&stripe).await?;
let o: Order<Shipped> = o.ship();
```

**Bug-uri eliminate:** TOATE statusurile invalide, flow greșit, dublă procesare — la compilare.

---

## 3. Handler access — fără vs cu Capability-Based

### ❌ Fără standards (AppState peste tot)

```rust
// main.rs
#[derive(Clone)]
struct AppState {
    db: PgPool,
    stripe_key: String,
    admin_emails: Vec<String>,
    // handler-ele au ACCES LA TOT
}

// handlers/checkout.rs
async fn checkout_handler(State(app): State<AppState>) -> Response {
    // Handlerul de checkout POATE:
    app.db.execute("DELETE FROM users");        // ❌ Șterge toți userii
    app.admin_emails.push("hacker@evil.com");   // ❌ Adaugă admini
    // Nu ar trebui să poată face asta, dar poate
}
```

**Problemă:** Niciun handler nu e restricționat. Un bug în `checkout_handler` poate șterge useri sau adăuga admini. **Zero izolare.**

### ✅ Cu standards (Capability-Based)

```rust
// state.rs — Fiecare handler primește DOAR ce-i trebuie
pub struct AuthState {
    pub auth: Arc<dyn AuthRepo>,   // Doar auth
    pub renderer: RenderService,
}

pub struct CartState {
    pub cart: Arc<dyn CartRepo>,   // Doar coș
    pub products: Arc<dyn ProductRepo>,
    pub auth: Arc<dyn AuthRepo>,
}

pub struct OrderState {
    pub orders: Arc<dyn OrderRepo>,  // Doar comenzi + plăți
    pub cart: Arc<dyn CartRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub auth: Arc<dyn AuthRepo>,
}

// handlers/checkout.rs
async fn checkout_handler(State(s): State<OrderState>) -> Response {
    // ✅ Handlerul POATE DOAR:
    s.cart.get_cart(&sid).await;        // OK — coș
    s.orders.place_order(...).await;     // OK — comenzi
    s.payment.create_checkout(...).await; // OK — plăți
    
    // ❌ Handlerul NU POATE:
    // s.db — nici măcar nu există
    // s.products — nu există în OrderState
    // s.admin_emails — nu există
    // ERROR: no field `db` on `OrderState` — LA COMPILARE!
}
```

**Bug-uri eliminate:** Acces neautorizat la date, escalation, daune colaterale — la compilare.

---

## 4. POST handler — fără vs cu PRG Pattern

### ❌ Fără standards (POST → HTML direct)

```rust
async fn checkout_handler(State(s): State<OrderState>, body: String) -> Html<String> {
    let order = s.orders.place_order(form).await.unwrap();
    let html = format!("<h1>Comanda {} a fost plasată!</h1>", order.id);
    Html(html)
    // ❌ F5 → POST din nou → încă o comandă
    // ❌ Dublu click → 2 comenzi
    // ❌ Back + Enter → comanda duplicată
}
```

**Bug-uri:** Comenzi duplicate, plăți duble, stoc epuizat de 2×.

### ✅ Cu standards (PRG: Post → Redirect → Get)

```rust
async fn checkout_handler(State(s): State<OrderState>, body: String) -> Response {
    let order = match s.orders.place_order(form).await {
        Ok(o) => o,
        Err(e) => return (StatusCode::FOUND, [
            ("Location", format!("/checkout?error={}", e)),
        ]).into_response(),
    };
    
    // ✅ POST → 302 → GET → F5 face GET, nu re-POST
    (StatusCode::FOUND, [
        ("Location", format!("/success?order_id={}", order.id)),
        ("Set-Cookie", "cart=; Max-Age=0"), // Golește coșul
    ]).into_response()
}

// GET /success — sigur, idempotent
async fn success_page(Query(q): Query<SuccessQuery>) -> Html<String> {
    let html = format!("<h1>Comanda {} confirmată!</h1>", q.order_id);
    Html(html)
    // ✅ F5 → GET → inofensiv
    // ✅ Dublu click → primul face POST, al doilea e 404 (coș gol)
    // ✅ Back + Enter → GET → încă o pagină de succes, nu încă o comandă
}
```

**Bug-uri eliminate:** Comenzi duplicate, plăți duble, stoc incorect.

---

## 5. Preț — fără vs cu Newtype + Overflow Protection

### ❌ Fără standards (i32 / f64 direct)

```rust
fn calculate_total(qty: i32, price: i32) -> i32 {
    qty * price
    // ❌ overflow: 100000 * 100000 = 1410065408 (la i32)
    // ❌ negativ: -1 * 100 = -100
    // ❌ f64: 0.1 + 0.2 = 0.30000000000000004
}

// Fără newtype, poți confunda prețul cu cantitatea:
fn process_order(price: i32, qty: i32) {
    // Care e prețul și care e cantitatea? Tipurile nu spun nimic.
}
```

**Bug-uri:** Overflow, prețuri negative, erori de rotunjire, confuzie între unități.

### ✅ Cu standards (Newtype + Parse)

```rust
/// Prețul în bani (1/100 dintr-un leu). GARANTAT:
/// - Strict pozitiv (> 0)
/// - Maximum 10.000 lei (1.000.000 bani)
/// - Zero floating point errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Price(i32);

impl Price {
    pub fn new(bani: i32) -> Result<Self, PriceError> {
        if bani <= 0 { return Err(PriceError::Negative); }
        if bani > 1_000_000 { return Err(PriceError::TooLarge); }
        Ok(Price(bani))
    }
    
    /// Înmulțire cu verificare de overflow (i64 intermediar)
    pub fn total(qty: u32, unit: Price) -> Result<Self, PriceError> {
        let total = (qty as i64) * (unit.0 as i64);
        if total > i32::MAX as i64 { return Err(PriceError::Overflow); }
        if total <= 0 { return Err(PriceError::Negative); }
        Ok(Price(total as i32))
    }
}

// ✅ Nu poți confunda prețul cu cantitatea:
fn process_order(price: &Price, qty: &Quantity) {
    // Tipurile sînt diferite — le poți da în ordinea GREȘITĂ
    // process_order(qty, price) // ❌ ERROR: expected &Price, found &Quantity
}

// ✅ Protejat la overflow:
let p = Price::new(100_000).unwrap();
assert!(Price::total(100_000, p).is_err()); // Overflow detectat!

// ✅ Fără erori de rotunjire:
let p = Price::new(24999).unwrap();
assert_eq!(p.as_lei_str(), "249.99"); // Exact, nu 249.98999999
```

**Bug-uri eliminate:** Overflow, prețuri negative, erori de rotunjire, confuzie între tipuri.

---

## 6. SQL — fără vs cu Query Parameterized

### ❌ Fără standards (concatenare string)

```rust
fn get_product(name: &str) -> Product {
    let sql = format!("SELECT * FROM products WHERE name = '{}'", name);
    // "name = 'Telefon'" → OK
    // "name = \"' OR 1=1 --\"" → TOATE produsele furate 🚨
    // "name = \"'; DELETE FROM products; --\"" → TOATE produsele șterse 🚨
    db.query(&sql)
}
```

**Bug-uri:** SQL injection — clasicul #1 OWASP Top 10.

### ✅ Cu standards (SQLx parametrizat)

```rust
async fn get_product(pool: &PgPool, name: &str) -> Result<Product, Error> {
    sqlx::query_as::<_, Product>(
        "SELECT * FROM products WHERE name = $1"
    )
    .bind(name)
    .fetch_one(pool)
    .await
    // ✅ $1 e întotdeauna valoare, nu SQL
    // "'; DELETE FROM products; --" → căutat în DB, nu executat ca SQL
    // OR 1=1 → căutat literal, nu interpretat
}
```

**Bug-uri eliminate:** TOATE formele de SQL injection — garantat prin protocol.

---

## Rezumat — Impactul standardelor

| Aspect | Fără standards | Cu standards | Bug-uri eliminate |
|--------|---------------|--------------|-------------------|
| **Email** | `String` — orice merge | `Email::parse()` — garantat valid | Email gol, XSS, caractere interzise |
| **Status comandă** | `String` — orice merge | `Order<Pending>` — flow garantat | Stări invalide, flow greșit |
| **Handler** | `AppState` — totul vizibil | `OrderState` — doar ce trebuie | Acces neautorizat, escalation |
| **POST** | HTML direct — F5 = duplicat | 302 redirect — F5 = GET sigur | Comenzi duplicate, plăți duble |
| **Preț** | `i32` — overflow, negativ | `Price::new()` — verificat | Overflow, prețuri negative, rotunjire |
| **SQL** | `format!("...{}...")` — injection | `$1` bind — garanta safe | SQL injection (OWASP #1) |
| **Testare** | Cazuri manuale, 3-5 exemple | Property-based, 10k cazuri | Edge cases, overflow, comutativitate |

### Regula de aur

> **Tipurile fac bug-urile imposibile.**
> **Testele le găsesc pe cele care au mai rămas.**
> **Standarde spun ce să cauți.**
> **Filosofia spune de ce.**