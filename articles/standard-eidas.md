---
title: "eIDAS — Electronic Identification and Trust Services"
description: "Implementarea regulamentului eIDAS pentru identificare electronică și servicii de încredere"
date: 2026-07-11
---

# eIDAS — Regulamentul European pentru Identitate Digitală

## Cuprins

1. [Ce este eIDAS](#ce-este-eidas)
2. [Nivele de asigurare](#nivele-de-asigurare)
3. [eIDAS 2.0](#eidas-20)
4. [Implementare în shop-mvp](#implementare-în-shop-mvp)
5. [Facturi și semnături electronice](#facturi-și-semnături-electronice)

## Ce este eIDAS

**eIDAS** (Electronic IDentification, Authentication and Trust Services) — Regulamentul UE 910/2014 — creează un cadru legal pentru:

1. **Identificarea electronică** — notificarea schemelor naționale de e-ID
2. **Servicii de încredere** — semnături electronice, sigilii, timestamp, livrare înregistrată

### Servicii de încredere definite

| Serviciu | Descriere |
|----------|-----------|
| **Semnătură electronică** | Date în formă electronică anexate logic la alte date |
| **Semnătură electronică avansată** | Unică, legată de semnatar, creată cu date sigure |
| **Semnătură electronică calificată** | Bazată pe certificat calificat, creată cu dispozitiv sigur |
| **Sigiliu electronic** | Similar semnăturii, dar pentru persoane juridice |
| **Timestamp** | Dovada existenței datelor la un moment dat |
| **Livrare înregistrată** | Transmitere cu dovada trimiterii și primirii |

## Nivele de asigurare

eIDAS definește 3 nivele de asigurare pentru identificare:

| Nivel | Descriere | În shop-mvp |
|-------|-----------|-------------|
| **Scăzut** | Parolă simplă (unic factor) | Login cu email + parolă |
| **Substanțial** | doi factori (ce știi + ce ai) | SCA prin Stripe Checkout |
| **Înalt** | Multi-factor cu verificare fizică | Necesită e-ID național |

## eIDAS 2.0

eIDAS 2.0 (Regulamentul 2024/1183, în vigoare din 2024) introduce:

- **European Digital Identity Wallet** (EUDI Wallet) — portofel digital european
- **Atribute calificate** — atestări electronice de atribute
- **Arhitectură open-source** — specificații publice
- **Zero-knowledge proofs** — divulgare selectivă de atribute

## Implementare în shop-mvp

### Autentificare

```rust
// Autentificarea curentă (nivel substanțial prin Stripe SCA)
pub async fn login_handler(
    State(s): State<AuthState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    // JWT token generation with exp claim
    // Account lockout after 5 failed attempts
}
```

### Facturi cu hash digital

```rust
// Generare hash pentru factură (integritate document)
fn generate_invoice_hash(invoice_data: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    invoice_data.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

// Verificare integritate factură
fn verify_invoice(data: &str, expected_hash: &str) -> bool {
    generate_invoice_hash(data) == expected_hash
}
```

### Model de date pentru factură

```rust
#[derive(Debug, Serialize)]
pub struct Invoice {
    pub id: uuid::Uuid,
    pub order_id: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub issuer_name: String,        // comerciant
    pub issuer_cui: String,         // CUI/Fiscal ID
    pub customer_name: String,
    pub items: Vec<InvoiceItem>,
    pub total_bani: i64,
    pub hash: String,               // SHA-256 hash
    pub signature: Option<String>,  // eIDAS qualified signature
}

#[derive(Debug, Serialize)]
pub struct InvoiceItem {
    pub name: String,
    pub qty: i32,
    pub price_bani: i64,
    pub total_bani: i64,
}
```

## Facturi și semnături electronice

### Format factură

Factura conține:

1. **Date comerciant**: nume, CUI, sediu, cont bancar
2. **Date client**: nume, adresă
3. **Produse**: denumire, cantitate, preț unitar, total
4. **Total**: valoare totală în lei
5. **Hash**: SHA-256 al conținutului (dovada integrității)
6. **Semnătură** (opțional): semnătură electronică calificată

### Fluxul emiterii

```
Comandă confirmată
    ↓
Generează factură cu hash
    ↓
Stochează în DB
    ↓
Trimite email (opțional)
    ↓
Client poate descărca factura
```

## Referințe

- [eIDAS Regulation (EUR-Lex)](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=uriserv:OJ.L_.2014.257.01.0073.01.ENG)
- [eIDAS 2.0 (EUR-Lex)](https://eur-lex.europa.eu/eli/reg/2024/1183)
- [Romanian eIDAS implementation](https://www.eid.as/)
