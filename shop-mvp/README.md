# 🛒 Shop MVP — Marketplace Minimal

Un marketplace MVP construit cu Rust/Axum, arhitectură modulară pe LEGO-uri.

```
╔══════════════════════════════════════════════╗
║              🛒  SHOP MVP                    ║
║                                              ║
║   ┌─────────┐  ┌──────┐  ┌───────────────┐  ║
║   │ Produse  │  │ Coș  │  │  Autentificare│  ║
║   │ Catalog  │→│ Persis-│→│  JWT + Argon2 │  ║
║   │ 24/page  │  │ tent  │  │  (signup/    │  ║
║   │ Căutare  │  │ (PG)  │  │   login)     │  ║
║   └────┬─────┘  └───┬───┘  └──────┬───────┘  ║
║        │            │             │           ║
║        ▼            ▼             ▼           ║
║   ┌──────────────────────────────────────┐    ║
║   │           Checkout + Stripe          │    ║
║   │   Comenzi → Plată → Status          │    ║
║   └──────────────────────────────────────┘    ║
║                                              ║
║   ┌──────────────────────────────────────┐    ║
║   │           Admin Panel                │    ║
║   │   Produse CRUD / Comenzi / Status   │    ║
║   └──────────────────────────────────────┘    ║
╚══════════════════════════════════════════════╝
```

## 🚀 Quick Start

```bash
# Pornește cu default-uri (dev)
DATABASE_URL="postgresql://postgres:123123@localhost:5432/test" \
JWT_SECRET="test-secret" \
STRIPE_SECRET_KEY="sk_test_..." \
cargo run -p shop-mvp
```

Accesează **http://localhost:3001**

## 🧱 Arhitectură — LEGO Modules

```
shop-mvp/                         ← Aplicația principală
├── src/main.rs                   ← ~850 lines, routes + handlers
├── templates/                    ← Tera templates (TailwindCSS)
│   ├── base.html                 ← Navbar + auth logic (JS)
│   ├── index.html                ← Landing page
│   ├── products.html             ← Grid de produse
│   ├── product_detail.html       ← Detalii + add to cart
│   ├── cart.html                 ← Coș + checkout button
│   ├── checkout.html             ← Formular livrare
│   ├── orders.html               ← Istoric comenzi
│   ├── success.html              ← Confirmare plată
│   ├── login.html / signup.html  ← Auth
│   ├── search.html               ← Căutare
│   └── admin_*.html              ← Admin panel
├── static/style.css              ← Stiluri custom
└── Cargo.toml

libs/                              ← LEGO-uri reutilizabile 🧱
├── rust-auth/                     ← Autentificare
│   ├── User + JWT (jsonwebtoken)
│   ├── Argon2 password hashing
│   └── PgAuthRepo (PostgreSQL)
│
├── rust-cart/                     ← Coș de cumpărături
│   ├── CartRepo trait
│   ├── PgCartRepo (session-based)
│   └── assign_to_user() după login
│
├── rust-marketplace-products/     ← Catalog produse
│   ├── ProductRepo trait
│   ├── Paginare, căutare, slug
│   └── CategoryService (injectat)
│
├── rust-marketplace-orders/       ← Comenzi
│   ├── OrderRepo trait
│   ├── Status workflow
│   └── Payment status tracking
│
├── rust-payment/                  ← Plăți Stripe
│   ├── PaymentRepo trait
│   └── Stripe Checkout API (reqwest)
│
└── rust-slug/                     ← Slug generation helper
```

## ✨ Features

### Utilizator
- ✅ Catalog produse cu paginare (24/page)
- ✅ Căutare full-text
- ✅ Coș persistent (PostgreSQL, sesiuni anonime)
- ✅ Autentificare (JWT + argon2)
- ✅ Checkout cu formular de livrare
- ✅ Plată prin Stripe Checkout
- ✅ Istoric comenzi cu status
- ✅ Status plată (unpaid → paid)

### Admin
- ✅ CRUD produse (adaugă, editează, șterge)
- ✅ Gestionare comenzi (status workflow)
- ✅ Validare: statusurile avansate necesită plată
- ✅ Roluri: doar `role='admin'` are acces

### Securitate
- ✅ Parole hash-uite cu Argon2
- ✅ JWT cu expiry (7 zile)
- ✅ Preț citit din DB, nu de la client
- ✅ Autorizare pe DB (revocare instantanee)
- ✅ Token în query param + Bearer header

## 🔧 Tech Stack

| Component | Tech |
|---|---|
| Web framework | Axum 0.8 |
| Database | PostgreSQL 18 + SQLx 0.9 |
| Auth | JWT (jsonwebtoken) + Argon2 |
| Templates | Tera 2.0 (Jinja2-like) |
| Frontend | TailwindCSS (CDN) + vanilla JS |
| Payments | Stripe Checkout API |
| Async | Tokio (full) |

## 📦 Dependencies

```toml
[dependencies]
axum = "0.8"          # web framework
sqlx = "0.9"          # PostgreSQL driver
tera = "2"            # template engine
jsonwebtoken = "9"    # JWT
argon2 = "0.5"        # password hashing
reqwest = "0.12"      # Stripe API client
tower-http = "0.7"    # CORS, trace, static files
tokio = "1.52"        # async runtime
```

## 🧪 Teste

```bash
# Teste unitare (fără DB)
cargo test -p rust-auth
cargo test -p rust-cart
cargo test -p rust-payment
cargo test -p rust-marketplace-orders
cargo test -p rust-marketplace-products

# Teste compilare
cargo check -p shop-mvp
```

## 🚀 Deployment (Cloud Run)

```bash
# Build + push
gcloud builds submit --tag europe-west1-docker.pkg.dev/.../myapp:$(date +%s)

# Deploy
gcloud run deploy shop-app --image=... --region=europe-west1 \
  --allow-unauthenticated \
  --set-env-vars="DATABASE_URL=...,JWT_SECRET=...,STRIPE_SECRET_KEY=..."
```

---

**MVP construit cu** 🧱 LEGO architecture — fiecare modul independent, testabil, și interschimbabil.
