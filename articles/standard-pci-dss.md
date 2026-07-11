# PCI DSS — Ghid pentru plăți sigure cu cardul

> *Payment Card Industry Data Security Standard — ce trebuie să respecte un magazin online care procesează plăți cu cardul prin Stripe.*

---

## 1. Ce este PCI DSS

PCI DSS este un standard de securitate creat de consorțiul cardurilor de plată (Visa, Mastercard, American Express, Discover, JCB). Se aplică ORICĂREI entități care stochează, procesează sau transmite date ale cardurilor de plată.

**Niveluri de conformitate** (în funcție de volumul tranzacțiilor):
- **Nivel 1**: > 6 milioane tranzacții/an — audit anual + scanare
- **Nivel 2**: 1-6 milioane — chestionar anual + scanare
- **Nivel 3**: 20.000-1 milion e-commerce — chestionar + scanare
- **Nivel 4**: < 20.000 e-commerce — chestionar + scanare

## 2. De ce Stripe simplifică PCI DSS

Stripe este **PCI DSS Level 1 compliant** (cel mai înalt nivel). Când folosești Stripe:
- **Nu stochezi** numere de card pe serverul tău
- **Nu vezi** codul CVV
- **Nu procesezi** direct datele cardului

**Ce rămâne în sarcina ta:**

| Domeniu | Cerință | Ești responsabil? |
|---------|---------|------------------|
| Stocare numere card | Niciunul (le ține Stripe) | ❌ |
| Transmisie date card | HTTPS + TLS 1.2+ | ✅ |
| Acces la sistem | Control acces, autentificare | ✅ |
| Logging | Monitorizare acces | ✅ |
| Testare | Scanări periodice | ✅ |
| Politici | Documentație securitate | ✅ |
| Furnizori terți | Contract cu Stripe | ✅ |

## 3. Cerințe aplicabile (SAQ A — cel mai simplu)

Pentru magazine care folosesc Stripe (redirect către Stripe pentru plată):

| # | Cerință | Implementare |
|---|---------|-------------|
| 1.1 | Firewall între rețele | Cloud Run oferă izolare |
| 2.1 | Fără parole implicite | ✅ |
| 2.2 | Configurație securizată | ✅ |
| 3.1 | Nu stoca date card | ✅ (Stripe le ține) |
| 4.1 | Criptare în tranzit | ✅ (HTTPS + TLS 1.2+) |
| 6.1 | Actualizări de securitate | 🟡 De verificat |
| 7.1 | Control acces restrictiv | ✅ (capability-based) |
| 8.1 | Autentificare puternică | ✅ (JWT + Argon2) |
| 9.1 | Restricționare acces fizic | N/A (Cloud) |
| 10.1 | Logging și monitorizare | ✅ |
| 11.1 | Testare periodică | 🟡 De stabilit |
| 12.1 | Politică de securitate | 🟡 De scris |

## 4. Implementări specifice

### 4.1 Logging acces la date sensibile

```rust
// Loghează orice acces la API-ul de comenzi
async fn log_access_to_orders(
    db: &PgPool,
    user_id: Uuid,
    action: &str,
    ip: IpAddr,
) {
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, ip_address)
         VALUES ($1, $2, 'order', $3, $4)"
    )
    .bind(user_id)
    .bind(action)
    .bind(Uuid::new_v4())
    .bind(ip.to_string())
    .execute(db).await.ok();
}
```

### 4.2 Scanare vulnerabilități

```bash
# Scanare automată săptămânală
# Folosește un ASV (Approved Scanning Vendor)
# Ex: Qualys, Trustwave, SecurityMetrics
docker run --rm vulners/nmap -sV -p 3001 localhost --script vulners
```

## 5. Checklist PCI DSS pentru shop-mvp

| # | Cerință | Status | Efort |
|---|---------|--------|-------|
| 1 | HTTPS + TLS 1.2+ ✅ | — |
| 2 | Control acces restrictiv ✅ | — |
| 3 | Autentificare puternică ✅ | — |
| 4 | Politici de securitate 🟡 | 1 zi |
| 5 | Testare periodică 🟡 | 1 zi |
| 6 | Contract Stripe ✅ | — |
| 7 | SAQ A completat 🟡 | 1 zi |
| **Total** | **3 de implementat** | **~3 zile** |
