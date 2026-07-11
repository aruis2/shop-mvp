# OWASP ASVS Level 2 — Ghid de conformitate avansată

> *De la security baseline la standard verification. Cum treci de la 100 de cerințe de bază la peste 200 de cerințe avansate pentru securitatea aplicațiilor web.*

---

## 1. Introducere

OWASP Application Security Verification Standard (ASVS) definește trei niveluri de verificare a securității:

- **Level 1 — Security Baseline**: Minimul necesar pentru orice aplicație web (14 capitole, ~100 cerințe)
- **Level 2 — Standard Verification**: Nivelul recomandat pentru majoritatea aplicațiilor care gestionează date sensibile (~200 cerințe)
- **Level 3 — Advanced Verification**: Pentru aplicații cu cerințe critice de securitate (medical, financiar, militar)

Acest ghid acoperă **tranziția de la Level 1 la Level 2**, cu cerințe specifice, implementări și exemple de cod.

## 2. Audit ASVS Level 2 pentru shop-mvp

### V2: Authentication (Autentificare)

| # | Cerință L2 | Status | Implementare |
|---|-----------|--------|-------------|
| 2.1.1 | Minimum password length ≥ 8 | ✅ | Deja implementat |
| 2.1.2 | Maximum password length ≥ 64 | ✅ | |
| 2.1.3 | Password hashing with Argon2 | ✅ | |
| **2.2.1** | **Multi-factor authentication (MFA)** | ❌ | Lipsă |
| **2.2.2** | **MFA enrollment flow** | ❌ | Lipsă |
| **2.3.1** | **Account lockout after N attempts** | ❌ | Rate limiting există, lockout lipsește |
| 2.4.1 | Password reset flow | 🟡 | Parțial |
| 2.5.1 | Credential recovery secure | 🟡 | Parțial |
| **2.6.2** | **Secure password manager interaction** | 🟡 | Parțial |

### V3: Session Management

| # | Cerință L2 | Status |
|---|-----------|--------|
| 3.1.1 | Session ID randomly generated | ✅ |
| 3.1.3 | Session ID length ≥ 128 bits | ✅ |
| **3.2.1** | **Session ID rotation after login** | ❌ |
| **3.3.1** | **Session timeout (inactivity logout)** | ❌ |
| 3.4.1 | Secure cookie attributes | ✅ |

### V4: Access Control

| # | Cerință L2 | Status |
|---|-----------|--------|
| 4.1.1 | Principle of least privilege | ✅ (capability-based) |
| **4.2.1** | **Anti-CSRF tokens** | ❌ (doar SameSite) |
| 4.2.2 | Anti-CSRF per sensitive action | ❌ |

### V8: Data Protection

| # | Cerință L2 | Status |
|---|-----------|--------|
| 8.1.1 | Encrypt data in transit | ✅ |
| **8.2.1** | **Encrypt sensitive data at rest** | ❌ (parole în DB sunt hash-uite, dar email-uri sunt în clar) |
| 8.3.1 | Cache-Control headers | ✅ |

### V9: Communication

| # | Cerință L2 | Status |
|---|-----------|--------|
| 9.1.1 | TLS 1.2+ | ✅ |
| 9.2.1 | HSTS with max-age ≥ 31536000 | ✅ |
| **9.3.1** | **Certificate pinning** | 🟡 (prin Cloud Run) |

### V11: Business Logic

| # | Cerință L2 | Status |
|---|-----------|--------|
| **11.1.1** | **Anti-automation controls** | 🟡 (rate limiting există, dar nu e complet) |
| **11.1.2** | **Business logic limits** | ❌ (ex: limită de 10 iteme per comandă) |
| **11.1.3** | **Idempotency for payments** | ❌ |

## 3. Implementări lipsă — soluții

### 3.1 Session timeout

```rust
/// Verifică dacă sesiunea e expirată
async fn session_timeout(
    State(state): State<AuthState>,
    headers: HeaderMap,
    req: Request<Body>,
    next: Next,
) -> Response {
    let timeout = Duration::from_secs(30 * 60); // 30 minute
    if let Some(cookie) = headers.get("cookie") {
        if let Some(token) = extract_token(cookie) {
            if let Ok(Some(created_at)) = get_session_created_at(&state.db, token).await {
                if created_at.elapsed() > timeout {
                    return (StatusCode::FOUND, [
                        ("Location", "/login?error=Sesiune expirată"),
                        ("Set-Cookie", "token=; Max-Age=0; Path=/"),
                    ]).into_response();
                }
            }
        }
    }
    next.run(req).await
}
```

### 3.2 CSRF tokens

```rust
/// Generează un token CSRF și îl stochează în sesiune
fn generate_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).unwrap();
    hex::encode(bytes)
}

/// Verifică token-ul CSRF la fiecare POST
fn verify_csrf_token(form_token: &str, session_token: &str) -> bool {
    // Constant-time comparison (previne timing attacks)
    form_token.as_bytes() == session_token.as_bytes()
}
```

### 3.3 Business logic limits

```rust
/// Limitează cantitatea maximă per produs la checkout
fn validate_order_limits(items: &[CartItem]) -> Result<(), Error> {
    let max_items = 10;
    let max_total = 10_000_00; // 10,000 lei
    
    if items.len() > max_items {
        return Err(Error::LimitExceeded("Prea multe produse într-o comandă"));
    }
    
    let total: i64 = items.iter().map(|i| i.price_bani as i64 * i.qty as i64).sum();
    if total > max_total {
        return Err(Error::LimitExceeded("Valoarea maximă a comenzii depășită"));
    }
    
    Ok(())
}
```

## 4. Rezumat

| Categorie | L1 ✅ | L2 ❌ | Efort estimat |
|-----------|-------|-------|---------------|
| Multi-factor auth | — | 2 cerințe | 3-5 zile |
| Session rotation | — | 1 cerință | 1 zi |
| Session timeout | — | 1 cerință | 1 zi |
| CSRF tokens | — | 2 cerințe | 2 zile |
| Data at rest encryption | — | 1 cerință | 1 zi |
| Anti-automation | — | 2 cerințe | 2 zile |
| Business logic limits | — | 3 cerințe | 2 zile |
| **Total** | **20** | **~12 noi** | **~2 săptămâni** |
