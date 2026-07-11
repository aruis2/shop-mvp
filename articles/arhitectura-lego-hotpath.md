# Arhitectura LEGO vs Hot Path — Un model hibrid pentru performanță și securitate

## Introducere

În ingineria software, există un conflict permanent între modularitate și performanță.
Modulele mici și izolate (LEGO) sunt excelente pentru securitate, testare și mentenanță, dar
adaugă overhead. Monoliții sunt rapizi, dar greu de întreținut și securizat.

Acest articol prezintă un model hibrid care combină avantajele ambelor abordări:
**LEGO la development, inline la runtime**.

---

## 1. Problema dynamic dispatch în Rust

Când folosim `Arc<dyn Trait>` pentru a injecta dependințe, fiecare apel de metodă
trece printr-un vtable (virtual table):

```rust
// LEGO — dynamic dispatch
trait PaymentRepo {
    async fn refund(&self, id: &str) -> Result<()>;
}

struct AppState {
    payment: Arc<dyn PaymentRepo>,  // ← vtable pointer
}

// Fiecare apel costă ~5-10ns în plus
s.payment.refund(pid).await?;
```

Pentru un request individual, 10ns e neglijabil. Dar la 10.000 req/s,
devine 100µs — suficient să simți diferența pe un hot path.

---

## 2. Abordarea clasică (monolit)

```
┌─────────────────────────────────────────┐
│             App (monolit)                │
│                                          │
│  fn refund() {                           │
│      // totul aici, direct               │
│      // Stripe API call                  │
│      // fără abstractizare               │
│  }                                       │
│                                          │
│  ✅ Rapid (inline)                       │
│  ❌ Intestabil (nu poți mock Stripe)     │
│  ❌ Insecurizat (chei în același loc)   │
│  ❌ AI productivitate scăzută           │
└─────────────────────────────────────────┘
```

---

## 3. Abordarea LEGO pură

```
┌─────────────────────────────────────────┐
│  App → PaymentRepo trait → StripePayment│
│                                          │
│  ✅ Testabil (poți injecta MockPayment)  │
│  ✅ Securizat (izolare prin trait)      │
│  ✅ AI productivitate (module mici)     │
│  ❌ Overhead vtable (~5-10ns per apel)  │
└─────────────────────────────────────────┘
```

---

## 4. Modelul hibrid — soluția

Separăm aplicația în două categorii:

```
⚡ HOT PATH (inline, rapid)           │  🧱 LEGO PATH (modular, securizat)
─────────────────────────────────────┼───────────────────────────────────
  Rulează la fiecare request          │  Rulează rar sau ține date sensibile
  Fără date sensibile                 │  Chei, tokeni, operații critice
  Testat abundent prin volum          │  Testat prin mock-uri
                                      │
  Exemple:                            │  Exemple:
  ✓ cart (adaugă/șterge)              │  ★ auth (JWT, parole)
  ✓ products (listare/căutare)        │  ★ payment (chei Stripe, refund)
  ✓ search                            │  ★ orders (workflow complex)
  ✓ url-normalizer                    │  ★ admin (operații rare)
```

---

## 5. Implementare practică

### 5.1 Feature flag în Cargo.toml

```toml
[features]
default = ["lego"]
lego = []        # development: dynamic dispatch, mock-uri
hot-path = []    # production: inline, monomorfizare
```

### 5.2 Tipuri condiționale

```rust
// libs/rust-payment/src/lib.rs

#[cfg(feature = "lego")]
pub type DynPaymentRepo = Arc<dyn PaymentRepo>;

#[cfg(not(feature = "lego"))]
pub type DynPaymentRepo = StripePayment;  // direct, fără vtable
```

### 5.3 AppState se adaptează

```rust
struct AppState {
    #[cfg(feature = "lego")]
    payment: Arc<dyn PaymentRepo>,

    #[cfg(not(feature = "lego"))]
    payment: StripePayment,  // inline, compilatorul face LTO
}
```

### 5.4 Construcția

```rust
#[cfg(feature = "lego")]
let payment: Arc<dyn PaymentRepo> = Arc::new(StripePayment::new(&key));

#[cfg(not(feature = "lego"))]
let payment = StripePayment::new(&key);
```

---

## 6. Câștigurile

| Metrică | LEGO (dev) | HOT PATH (prod) |
|---|---|---|
| Apel metodă | ~10ns (vtable) | ~1ns (inline) |
| Testabilitate | Excelentă (mock) | Bună (teste integrări) |
| Securitate | Izolare maximă | Aceeași (același cod) |
| AI productivitate | Maximă (module mici) | Aceeași |
| Timp compilare | ~5s | ~30s (LTO) |
| Dimensiune binar | ~10MB | ~8MB |

---

## 7. Reguli de decizie

### Când pui ceva pe HOT PATH

1. Rulează la **fiecare request** (middleware, cart, products)
2. **Nu conține date sensibile** (chei, tokeni, parole)
3. Este **bine testat prin volum** (mii de requesturi/zi)
4. Dacă se strică, **nu pierzi bani direct** (doar experiență utilizator)

### Când pui ceva pe LEGO PATH

1. Rulează **rar** (admin, refund, webhook)
2. **Conține date sensibile** (chei Stripe, JWT secret, parole)
3. Trebuie **izolat** de restul aplicației
4. Dacă se strică, **pierzi bani sau date**

---

## 8. Concluzie

Modelul hibrid LEGO + HOT PATH oferă:

| Dezvoltare | Producție |
|---|---|
| Module mici, testabile | Același cod, compilat inline |
| Mock-uri pentru orice | LTO elimină overhead-ul |
| AI vede tot modulul (50-100 linii) | Performanță de monolit |
| Securitate prin izolare | Securitate prin verificare |

> **Nu sacrifica modularitatea pentru performanță. Ci folosește compilatorul să elimine**
> **overhead-ul acolo unde contează.**

---

*Articol intern — arhitectură Shop MVP*
*GitHub Copilot (DeepSeek V4 Flash) — 2026-07-09*
