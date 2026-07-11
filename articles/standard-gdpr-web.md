# GDPR Compliance pentru Aplicații Web

> *Regulamentul General privind Protecția Datelor (GDPR) — cum să îl implementezi practic într-un magazin online fără avocați.*

---

## 1. Ce este GDPR

Regulamentul (UE) 2016/679 este în vigoare din 25 mai 2018. Se aplică ORICĂREI organizații care prelucrează date personale ale rezidenților UE, indiferent de locația organizației.

**Amendă maximă:** 20.000.000 EUR sau 4% din cifra de afaceri anuală globală (oricare e mai mare).

## 2. Principiile GDPR

1. **Legalitate, echitate, transparență** — Prelucrarea datelor trebuie să aibă o bază legală
2. **Limitarea scopului** — Datele colectate doar pentru scopuri specifice, explicite, legitime
3. **Minimizarea datelor** — Colectează doar ce e strict necesar
4. **Exactitatea** — Datele trebuie să fie corecte și actualizate
5. **Limitarea stocării** — Datele păstrate doar cât e necesar
6. **Integritate și confidențialitate** — Măsuri tehnice și organizatorice adecvate
7. **Responsabilitate (accountability)** — Poți demonstra conformitatea

## 3. Bazele legale pentru prelucrare

Pentru un magazin online:

| Scopul prelucrării | Bază legală | Necesar |
|-------------------|-------------|---------|
| Procesare comandă | Executarea unui contract | Da |
| Plată (Stripe) | Executarea unui contract | Da |
| Cont utilizator | Consimțământ | Da |
| Newsletter | Consimțământ (opt-in) | Opțional |
| Analytics | Interes legitim | Opțional |

## 4. Ce trebuie implementat

### 4.1 Politica de confidențialitate

Document care explică:
- Ce date colectăm (nume, email, adresă, telefon)
- De ce le colectăm (executare comandă, livrare)
- Cu cine le împărțim (Stripe pentru plată, curier pentru livrare)
- Cât timp le păstrăm (obligații fiscale: 10 ani pentru facturi)
- Ce drepturi are utilizatorul

### 4.2 Consimțământ cookie

```html
<!-- Banner cookie la prima vizită -->
<div id="cookie-banner" style="display:none;">
  <p>Acest site folosește cookie-uri esențiale pentru funcționare.
  <a href="/politica-confidentialitate">Detalii</a></p>
  <button onclick="acceptCookies()">Accept</button>
</div>
```

### 4.3 Drepturile utilizatorului

```rust
// API pentru ștergerea contului (Dreptul la ștergere - Art. 17)
async fn delete_account(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> Response {
    let user = authenticate(&headers, &state.auth).await?;
    
    // Anonimizează datele personale
    sqlx::query("UPDATE users SET email = 'deleted@' || id, name = 'Șters' WHERE id = $1")
        .bind(user.id)
        .execute(&state.db).await?;
    
    // Păstrează comenzile (obligații fiscale) dar anonimizate
    sqlx::query("UPDATE orders SET shipping_name = 'Șters', shipping_phone = '' WHERE user_id = $1")
        .bind(user.id)
        .execute(&state.db).await?;
    
    (StatusCode::FOUND, [("Location", "/")]).into_response()
}

// API pentru export date (Dreptul la portabilitate - Art. 20)
async fn export_data(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> Json<Value> {
    let user = authenticate(&headers, &state.auth).await?;
    
    let orders = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE user_id = $1")
        .bind(user.id)
        .fetch_all(&state.db).await?;
    
    Json(json!({
        "user": user,
        "orders": orders,
        "exported_at": Utc::now()
    }))
}
```

### 4.4 Notificarea breșelor de securitate (Art. 33-34)

```rust
// Log pentru breșe
async fn log_breach(
    db: &PgPool,
    incident_type: &str,
    affected_users: i32,
    description: &str,
) {
    sqlx::query(
        "INSERT INTO security_incidents (type, affected_users, description, notified)
         VALUES ($1, $2, $3, false)"
    )
    .bind(incident_type)
    .bind(affected_users)
    .bind(description)
    .execute(db).await.ok();
    
    // Trebuie notificată autoritatea în 72h
    // Trebuie notificați utilizatorii afectați
}
```

## 5. Checklist GDPR pentru shop-mvp

| # | Cerință | Status | Efort |
|---|---------|--------|-------|
| 1 | Politică de confidențialitate | ❌ | 2 zile |
| 2 | Banner cookie | ❌ | 1 zi |
| 3 | Dreptul la ștergere (API) | ❌ | 1 zi |
| 4 | Portabilitate date (API export) | ❌ | 1 zi |
| 5 | Log breșe de securitate | ❌ | 1 zi |
| 6 | Contract cu procesorii de date (Stripe, curier) | ❌ | 1 zi |
| 7 | Registru de prelucrare | ❌ | 1 zi |
| 8 | Minimizarea datelor colectate | ✅ | — |
| 9 | Criptare în tranzit (HTTPS) | ✅ | — |
| 10 | Control acces bazat pe roluri | ✅ | — |
| **Total** | **6 de implementat** | | **~1 săptămână** |
