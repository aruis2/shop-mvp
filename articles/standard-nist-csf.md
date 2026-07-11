# NIST Cybersecurity Framework — Ghid practic

> *National Institute of Standards and Technology Cybersecurity Framework — un cadru comprehensiv pentru gestionarea riscurilor de securitate cibernetică.*

---

## 1. Ce este NIST CSF

Publicat inițial în 2014, actualizat în 2024 (CSF 2.0), NIST Cybersecurity Framework oferă un **limbaj comun** pentru înțelegerea, gestionarea și comunicarea riscurilor de securitate cibernetică. Nu este o certificare obligatorie, dar e **standardul de facto** în SUA și recunoscut global.

**Structură:** 6 funcții → 22 categorii → 107 subcategorii

## 2. Cele 6 funcții

### 2.1 Identify (Identificare)

| Categorie | În shop-mvp |
|-----------|-------------|
| Asset Management | Servere, DB, cod, domenii |
| Business Environment | Roluri, misiune, părți interesate |
| Governance | Politici, proceduri |
| Risk Assessment | STRIDE + threat model |
| Risk Management Strategy | Prioritizare riscuri |
| Supply Chain | Stripe, Cloud Run, Docker |

**Implementare:**

```rust
// Inventar automat al rutelor expuse
fn list_all_endpoints() -> Vec<&'static str> {
    vec![
        "GET /", "GET /health", "GET /login", "POST /login",
        "GET /products", "GET /product/{slug}",
        "GET /cart", "POST /cart/add", "POST /cart/remove",
        "GET /checkout", "POST /checkout",
        "GET /orders", "POST /order/{id}/pay",
        "GET /admin", "POST /admin/product/new",
    ]
}
```

### 2.2 Protect (Protejare)

| Categorie | În shop-mvp |
|-----------|-------------|
| Identity Management | JWT, Argon2, capability-based ✅ |
| Awareness & Training | Documentație, manuale |
| Data Security | HTTPS, CSP, HSTS ✅ |
| Platform Security | Cloud Run, Docker ✅ |
| Technology Maintenance | Actualizări dependințe |

### 2.3 Detect (Detectare)

| Categorie | În shop-mvp |
|-----------|-------------|
| Anomalies & Events | Request timing, DB query counter ✅ |
| Continuous Monitoring | Logging ✅ |
| Detection Processes | Rate limiting alerts 🟡 |

```rust
// Detectare anomalii — request-uri anormal de lente
async fn detect_slow_requests(duration_ms: u64, path: &str) {
    if duration_ms > 1000 {
        tracing::warn!(
            target: "security",
            "Request anormal de lent: {} -> {}ms",
            path, duration_ms
        );
    }
}
```

### 2.4 Respond (Răspuns)

| Categorie | Implementare |
|-----------|-------------|
| Response Planning | Plan de incident 🟡 (lipsă) |
| Communications | Notificare utilizatori + autorități 🟡 |
| Analysis | Analiză post-incident 🟡 |
| Mitigation | Izolare, patch 🟡 |
| Improvements | Actualizări bazate pe lecții 🟡 |

### 2.5 Recover (Recuperare)

| Categorie | Implementare |
|-----------|-------------|
| Recovery Planning | Backup DB 🟡 (lipsă) |
| Improvements | Lecții învățate 🟡 |
| Communications | Revenire la normal 🟡 |

### 2.6 Govern (Guvernanță) — NOU în CSF 2.0

| Categorie | Implementare |
|-----------|-------------|
| Organizational Context | Contextul organizației |
| Risk Management Strategy | Strategie de risc |
| Roles & Responsibilities | Roluri definite |
| Policy | Politici de securitate |
| Oversight | Supraveghere |

## 3. Implementare practică

### 3.1 Plan de răspuns la incident

```rust
/// Structura unui incident de securitate
struct SecurityIncident {
    id: Uuid,
    detected_at: DateTime<Utc>,
    severity: IncidentSeverity,
    description: String,
    affected_systems: Vec<String>,
    affected_users: i32,
    status: IncidentStatus,
    resolution: Option<String>,
}

enum IncidentSeverity {
    Low,    // Scam, tentative
    Medium, // Atac reușit, date accesate
    High,   // Date compromise, serviciu întrerupt
    Critical, // Breșă majoră, autorități notificate
}

enum IncidentStatus {
    Detected,
    Analyzing,
    Containing,
    Eradicated,
    Recovered,
    Closed,
}
```

### 3.2 Backup DB automat

```bash
#!/bin/bash
# backup-db.sh — Programat în cron: 0 3 * * *
DATE=$(date +%Y-%m-%d)
BACKUP_DIR="/backups"
pg_dump -h localhost -U postgres test > "$BACKUP_DIR/test-$DATE.sql"
gzip "$BACKUP_DIR/test-$DATE.sql"
# Păstrează ultimele 30 de zile
find "$BACKUP_DIR" -name "test-*.sql.gz" -mtime +30 -delete
```

## 4. Matricea NIST CSF pentru shop-mvp

| Funcție | Categorie | Status | Efort |
|---------|-----------|--------|-------|
| Identify | Asset Management | ✅ | — |
| Identify | Risk Assessment | ✅ (STRIDE) | — |
| Protect | Identity Management | ✅ | — |
| Protect | Data Security | ✅ | — |
| Detect | Anomalies | 🟡 | 1 zi |
| Detect | Monitoring | ✅ | — |
| Respond | Response Planning | ❌ | 2 zile |
| Respond | Communications | ❌ | 1 zi |
| Recover | Backup | ❌ | 1 zi |
| Recover | Improvements | ❌ | 1 zi |
| Govern | Policy | ❌ | 2 zile |
| **Total** | **6 de implementat** | | **~1 săptămână** |
