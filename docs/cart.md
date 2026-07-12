# 🛒 Coșul de cumpărături — arhitectură

## 1. Concept: două tipuri de iteme

Coșul suportă **două categorii** de iteme, care coexistă în același tabel `cart_items`:

| Tip | `user_id` | `session_id` | Vizibil după logout | Persistență |
|-----|-----------|--------------|---------------------|-------------|
| **Public** (anonim) | `NULL` | browser session cookie | ✅ da | Doar pe browserul curent |
| **Privat** (al utilizatorului) | `UUID` (user_id) | `user_id.to_string()` | ❌ nu | Cross-browser, legat de cont |

### Reguli de bază

- Itemul public rămâne **întotdeauna public** — nu se "adoptă" la login.
- Itemul privat se leagă de `user_id` și poartă `session_id = user_id.to_string()` (ca să nu intre în conflict cu indexul unic al itemelor publice).
- La logout, itemele private **dispar** din coș — doar cele publice rămân.

---

## 2. Tabela `cart_items`

```sql
CREATE TABLE cart_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id TEXT NOT NULL,
    user_id UUID,           -- NULL pentru iteme publice
    product_slug TEXT NOT NULL,
    product_name TEXT NOT NULL,
    price_bani BIGINT NOT NULL,
    qty INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Indexuri unice

```sql
-- Itemele publice: unice per (session_id, product_slug, price_bani)
CREATE UNIQUE INDEX idx_cart_unique_session_product_price
ON cart_items (session_id, product_slug, price_bani);

-- Itemele private: unice per (user_id, product_slug, price_bani)
-- Index parțial — se aplică doar unde user_id IS NOT NULL
CREATE UNIQUE INDEX idx_cart_unique_user_product_price
ON cart_items (user_id, product_slug, price_bani)
WHERE user_id IS NOT NULL;
```

**De ce două indexuri?** Pentru că `ON CONFLICT` nu funcționează cu indexuri parțiale
(`WHERE user_id IS NOT NULL`). Așa că avem:
- Un index **total** pentru iteme publice (toate rândurile)
- Un index **parțial** pentru iteme private (doar unde `user_id` există)

---

## 3. Fluxuri principale

### 3.1 Adăugare în coș

Handler: `cart_add()` în `shop-mvp/src/handlers/cart.rs`
Implementare DB: `add_item()` în `libs/rust-cart/src/pg.rs`

#### Cazul anonim (fără `user_id`)
```
INSERT ... ON CONFLICT (session_id, product_slug, price_bani)
DO UPDATE SET qty = qty + EXCLUDED.qty
```
Folosește `ON CONFLICT` direct — upsert după session.

#### Cazul autentificat (cu `user_id`)
```
1. UPDATE cart_items SET qty = qty + $3, session_id = $4
   WHERE user_id = $1 AND product_slug = $2 AND price_bani = $5
2. Dacă niciun rând afectat → INSERT cu session_id = uid.to_string()
```

**Nu există "adopție"** — itemul public rămâne public, se creează un **nou** item privat separat.
`session_id` la itemele private = `user_id.to_string()` pentru a evita conflictul cu indexul
unic al itemelor publice (care ar fi aceleași valori session_id, product_slug, price_bani).

### 3.2 Vizualizare coș

Handler: `cart_page()` în `shop-mvp/src/handlers/cart.rs`

```
Dacă utilizator e autentificat:
  → get_cart_by_user(session_id, user_id)
     SELECT ... WHERE session_id = $1 OR user_id = $2
  → primește iteme publice (session_id match) + private (user_id match)

Dacă anonim:
  → get_cart(session_id)
     SELECT ... WHERE session_id = $1 AND user_id IS NULL
  → doar iteme publice (cele fără user_id)
```

Template-ul (`shop-mvp/templates/cart/cart.html`) afișează:
- **🔒 Coșul tău privat** — secțiune separată cu itemele care au `user_id`
- **📦 Produse din sesiunea anterioară** — itemele publice (fără `user_id`)
- Când utilizatorul e autentificat și are ambele tipuri → două butoane:
  - **"Cumpără doar privat"** → `/checkout?cart=private`
  - **"Cumpără tot"** → `/checkout`

### 3.3 Ștergere din coș

Handler: `cart_remove()` în `shop-mvp/src/handlers/cart.rs`
Implementare DB: `remove_item()` în `libs/rust-cart/src/pg.rs`

```sql
DELETE FROM cart_items WHERE id = $1
```

**Nu se mai verifică `session_id`** — itemele private au `session_id = user_id_string`,
care nu se potrivește cu cookie-ul browserului. Identifier-ul `id` (UUID) e suficient
ca măsură de securitate — UUID-urile sunt neghicibile.

### 3.4 Cantitate (update)

```sql
UPDATE cart_items SET qty = $2 WHERE id = $1
```

La fel ca ștergerea — doar după `id`, fără `session_id`.

### 3.5 Cross-browser

Când același utilizator se autentifică de pe un alt browser:
- `get_cart_by_user(session_id_2, user_id)` este apelat
- Găsește itemele private via `WHERE user_id = $2`
- Itemele publice sunt diferite per browser (fiecare are propriul `session_id`)

---

## 4. Date transmise la template (`cart_page`)

Handler-ul construiește un JSON cu:

| Câmp | Sursă | Descriere |
|------|-------|-----------|
| `cart_items` | Toate itemele | Lista completă (folosită doar pentru `length == 0`) |
| `private_items` | `user_id IS NOT NULL` | Itemele private |
| `public_items` | `user_id IS NULL` | Itemele publice |
| `total_lei` | Suma tuturor | Total general |
| `private_total_lei` | Suma private | Total privat |
| `public_total_lei` | Suma publice | Total sesiune |
| `has_private` | `!private_items.is_empty()` | Are iteme private? |
| `has_public` | `!public_items.is_empty()` | Are iteme publice? |
| `is_authenticated` | `user_auth.is_some()` | E autentificat? |
| `session_id` | cookie | ID sesiune browser |
| `added` | Query param `?added=1` | Mesaj flash "Produs adăugat" |
| `error` | Query param `?error=` | Mesaj flash eroare |

---

## 5. Securitate

- **`remove_item` / `update_qty`**: fără `session_id` — doar `id` (UUID).
  Risc neglijabil: UUID-urile sunt 128-bit random.
- **`get_cart`**: filtrează `AND user_id IS NULL` — itemele private nu se scurg după logout.
- **`add_item`**: itemele private au `session_id = uid.to_string()`, deci nu se ciocnesc
  de indexul `(session_id, product_slug, price_bani)` al itemelor publice.
- **Input validation**: `InputFactory::parse_slug`, `InputFactory::parse_qty`,
  `LogicFactory::verify_qty_in_range`, `LogicFactory::verify_stock_available`.

---

## 6. Localizare în cod

| Componentă | Fișier |
|------------|--------|
| Trait `CartRepo` | `libs/rust-cart/src/lib.rs` |
| Implementare PostgreSQL | `libs/rust-cart/src/pg.rs` |
| Handlere HTTP (add, remove, page) | `shop-mvp/src/handlers/cart.rs` |
| Template | `shop-mvp/templates/cart/cart.html` |
| Models (`CartItem`, `Cart`, etc.) | `libs/rust-cart/src/models.rs` |

---

## 7. Testare

```bash
# Teste comportamentale (standard)
bash test-behavior.sh

# Teste de securitate/endpoint
bash test-curl.sh
```

Testele comportamentale acoperă fluxurile utilizator (inclusiv coș).
Nu uita să cureți DB-ul între testări manuale:
```bash
PGPASSWORD=123123 psql -h localhost -U postgres -d test -c "DELETE FROM cart_items;"
```
