# SPEC — Comportament complet shop-mvp (testabil cu curl)

> Fiecare rută, fiecare caz, fără excepții.
> Dacă nu e în spec, e un bug.

---

## 1. Pagini generale

### `GET /`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără parametri | 200 | Home page |
| `?error=msg` | 200 | Home page cu mesaj eroare |

### `GET /health`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără parametri | 200 | `"OK"` (text) |

### `GET /login`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Neautentificat | 200 | Formular login |
| `?redirect=/orders` | 200 | Formular + redirect ascuns |
| `?error=msg` | 200 | Formular + eroare |
| Autentificat (cookie valid) | 200 | Redirect HTML (meta refresh la `/`) |
| Autentificat + `?redirect=/admin` | 200 | Redirect HTML la `/admin` |

### `GET /signup`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Neautentificat | 200 | Formular înregistrare |
| `?redirect=/orders` | 200 | Formular + redirect ascuns |
| `?error=msg` | 200 | Formular + eroare |
| Autentificat (cookie valid) | 200 | Redirect HTML (meta refresh la `/`) |

### `GET /me`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără cookie | 401 | `"Neautentificat"` |
| Cookie invalid | 401 | Mesaj eroare |
| Cookie valid (user normal) | 200 | JSON `{id, email, name, role}` |
| Cookie valid (admin) | 200 | JSON `{..., role: "admin"}` |

### `GET /logout`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără parametri | 302 | Redirect la `/` + șterge cookie |
| `?redirect=/products` | 302 | Redirect la `/products` + șterge cookie |
| Cu `Referer: /cart` | 302 | Redirect la `/cart` + șterge cookie |

### `POST /logout`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără parametri | 302 | Redirect la `/` + șterge cookie |
| `?redirect=/products` | 302 | Redirect la `/products` |

---

## 2. Autentificare

### `POST /login`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Email+parolă corecte | 302 | `Set-Cookie: token=...` → redirect |
| Email+parolă corecte + `redirect=/orders` | 302 | `Set-Cookie` + redirect la `/orders` |
| Email greșit | 400 | `"Invalid credentials"` (text) |
| Parolă greșită | 400 | `"Invalid credentials"` (text) |
| Body gol | 400 | `"Date invalide: ..."` |
| Body invalid (JSON malformat) | 400 | `"Date invalide: ..."` |
| Câmp lipsă (doar email) | 400 | `"Date invalide: ..."` |

### `POST /signup`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Date complete valide | 302 | `Set-Cookie: token=...` → redirect |
| Email deja existent | 302 | Redirect la `/signup?error=...` |
| Parolă prea scurtă | 302 | Redirect la `/signup?error=...` |
| Body gol | 302 | Redirect la `/signup?error=...` |
| Câmp lipsă | 302 | Redirect la `/signup?error=...` |
| `redirect=/products` + valid | 302 | Redirect la `/products` |

---

## 3. Produse

### `GET /products`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Produse în DB | 200 | Listă cu maxim 24 produse |
| Fără produse | 200 | Listă goală (cu mesaj) |
| `?page=1` | 200 | Pagina 1 |
| `?page=2` (dacă există) | 200 | Pagina 2 |
| `?page=999` (peste limită) | 200 | Pagina goală (sau ultima) |
| `?page=-1` | 200 | Tratat ca page 1 |
| `?page=abc` | 400 | Query string invalid (serde) |

### `GET /product/{slug}`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Slug valid existent | 200 | Pagină detaliu produs |
| Slug inexistent | 404 | `"Produs negăsit"` |
| Slug gol `/product/` | 301 | Redirect la `/product` (trailing slash) |
| Slug cu caractere speciale | 200/404 | Depinde de DB |
| Slug cu spații (URL encoded) | 200/404 | Depinde de DB |

### `GET /search`
| Caz | Status | Răspuns |
|-----|--------|---------|
| `?q=termen` + rezultate | 200 | Listă rezultate |
| `?q=termen` + 0 rezultate | 200 | Listă goală |
| `?q=termen&page=1` | 200 | Pagina 1 |
| `?q=termen&page=2` | 200 | Pagina 2 |
| Fără `?q=` | 400 | Query string invalid (serde) |
| `?q=` (gol) | 200 | Căutare cu query gol (toate produsele?) |

---

## 4. Coș

### `GET /cart`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără cookie/header | 200 | Coș gol (session "anon") |
| Cu cookie `session_id` valid | 200 | Coș cu iteme |
| Cu `X-Session-Id` header | 200 | Coș pe session respectiv |
| Cu `?session_id=x` | 200 | Coș pe session respectiv |
| `?error=msg` | 200 | Coș cu eroare |
| Session inexistent | 200 | Coș gol |

### `POST /cart/add`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Slug valid + qty | 302 | Redirect la referer + Set-Cookie session_id |
| Slug valid + fără qty | 302 | Default qty=1 + redirect |
| Slug valid + qty=0 | 302 | qty clamped la min=1 |
| Slug valid + qty>max | 302 | qty clamped la max |
| Slug inexistent | 302 | `?error=Produs negăsit` |
| Fără slug | 302 | `?error=Date invalide` |
| Produs fără preț | 302 | `?error=Produsul nu are preț` |
| JSON body valid | 302 | La fel ca form |
| Body gol | 302 | `?error=Date invalide` |
| Cu `Referer: /products` | 302 | Redirect la `/products` |
| Fără Referer | 302 | Redirect la `/products` (fallback) |

### `POST /cart/remove`
| Caz | Status | Răspuns |
|-----|--------|---------|
| `item_id` valid în coș | 302 | Redirect + item șters |
| `item_id` inexistent | 302 | `?error=Ștergere eșuată` |
| Fără `item_id` | 302 | `?error=Date invalide` |
| `item_id` UUID invalid | 302 | `?error=ID invalid` |
| Body gol | 302 | `?error=Date invalide` |

---

## 5. Comenzi

### `GET /checkout`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Session cu iteme | 200 | Formular checkout |
| Session goală/gol | 302 | `?error=Coșul e gol` la `/cart` |
| Session inexistent | 302 | `?error=...` la `/cart` |
| `?session_id=x` (cu iteme) | 200 | Formular checkout |

### `POST /checkout`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Formular complet + coș valid | 302 | Redirect la Stripe checkout URL |
| Formular complet + coș gol | 302 | `?error=Coșul e gol` la `/cart` |
| Fără session_id | 302 | `?error=...` |
| Câmpuri obligatorii lipsă | 302 | `?error=Date invalide` |
| Stripe eșuează | 302 | Redirect la `/orders` |

### `GET /orders`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Neautentificat (fără token) | 302 | Redirect la `/login?redirect=/orders` |
| Token invalid | 302 | Redirect la `/login?redirect=/orders` |
| Autentificat + fără comenzi | 200 | Listă goală |
| Autentificat + cu comenzi | 200 | Listă cu maxim 10 comenzi |
| `?token=xxx` (valid) | 200 | Autentificat via query param |
| `?token=xxx` (invalid) | 302 | Redirect la login |
| `?page=1` | 200 | Pagina 1 |
| `?page=2` | 200 | Pagina 2 |
| `?error=msg` | 200 | Pagină cu eroare |

### `POST /order/{id}/pay`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Neautentificat | 302 | `?error=Trebuie să fii autentificat` la `/login` |
| Token invalid | 302 | `?error=Token invalid` la `/login` |
| Comanda nu există | 302 | `?error=Comanda nu există` la `/orders` |
| Comanda altui user | 302 | `?error=Nu e comanda ta` la `/orders` |
| Deja plătită | 302 | `?error=Deja plătită` la `/orders` |
| Valid + Stripe OK | 302 | Redirect la Stripe checkout URL |
| Valid + Stripe fail | 302 | Redirect la `/orders` |

### `GET /success`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără `order_id` | 200 | Pagina success generică |
| `?order_id=uuid` (valid) | 200 | Pagina success + update payment_status |
| `?order_id=invalid` | 200 | Pagina success (uuid parse fail, ignorat) |

---

## 6. Admin

### `GET /admin`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 200 | Redirect HTML (JS) la login |
| Token invalid | 200 | Redirect HTML (JS) la login |
| Token valid + nu e admin | 200 | Redirect HTML (JS) la home cu eroare |
| Token valid + admin | 200 | Listă produse (maxim 25) |
| `?page=N` | 200 | Paginare |
| `?error=msg` | 200 | Eroare |

### `GET /admin/orders`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 200 | Redirect HTML la login |
| Token valid + admin | 200 | Listă comenzi (maxim 25) |

### `GET /admin/product/new`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 200 | Redirect HTML la login |
| Token valida + admin | 200 | Formular produs gol |

### `GET /admin/product/{slug}/edit`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 200 | Redirect HTML la login |
| Token valid + admin + slug valid | 200 | Formular cu datele produsului |
| Token valid + admin + slug inexistent | 404 | `"Produs negăsit"` |

### `GET /admin/logs`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 200 | Redirect HTML la login |
| Token valid + admin | 200 | Listă ultimele 100 query-uri |

### `POST /admin/product/new`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 200 | Redirect HTML la login |
| Token valid + admin + date valide | 302 | Redirect la admin |
| Token valid + admin + date invalide | 302 | `?error=...` la referer |
| Body gol | 302 | `?error=...` |

### `POST /admin/product/{slug}/edit`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 200 | Redirect HTML la login |
| Token valid + admin + date valide | 302 | Redirect la admin |
| Token valid + admin + date invalide | 302 | `?error=...` |
| Slug inexistent | 302 | `?error=...` |

### `POST /admin/product/{slug}/delete`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 200 | Redirect HTML la login |
| Token valid + admin + slug valid | 302 | Redirect + produs șters |
| Token valid + admin + slug inexistent | 302 | `?error=...` |
| **GET** (method not allowed) | 405 | `405 Method Not Allowed` |

### `POST /admin/order/{id}/status`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 200 | Redirect HTML la login |
| Token valid + admin + status valid | 302 | Redirect + status actualizat |
| Comanda inexistentă | 302 | `?error=Comanda negăsită` |
| Plata neefectuată + status "confirmed" | 302 | `?error=Comanda nu poate fi confirmată...` |
| Comanda expediată + status "cancelled" | 302 | `?error=Comanda nu poate fi anulată...` |
| **GET** | 405 | `405 Method Not Allowed` |

### `POST /admin/migrate-orders`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 401 | `"Admin: token lipsă"` |
| Token invalid | 401 | `"Admin: token invalid"` |
| Token valid + nu e admin | 403 | `"Admin: acces interzis"` |
| Token valid + admin | 200 | JSON `{"migrated": N}` |
| **GET** | 405 | `405 Method Not Allowed` |

---

## 7. Rute inexistente

| Cale | Status | Răspuns |
|------|--------|---------|
| `/nonexistent` | 404 | Pagină eroare 404 |
| `/produse` (română) | 404 | Pagină eroare 404 |
| `/cos` (română) | 404 | Pagină eroare 404 |
| `/admin/nonexistent` | 404 | Pagină eroare 404 |
| `/shop/nonexistent` | 404 | Pagină eroare 404 |

## 8. Trailing slash

| Cale | Status | Răspuns |
|------|--------|---------|
| `/products/` | 301 | Redirect la `/products` |
| `/cart/` | 301 | Redirect la `/cart` |
| `/search/` | 301 | Redirect la `/search` |
| `/login/` | 301 | Redirect la `/login` |
| `/signup/` | 301 | Redirect la `/signup` |
| `/checkout/` | 301 | Redirect la `/checkout` |
| `/admin/` | 301 | Redirect la `/admin` |
| `/orders/` | 301 | Redirect la `/orders` |
| `/success/` | 301 | Redirect la `/success` |
| `/shop/products/` | 301 | Redirect la `/shop/products` |
| `/` (rădăcina) | 200 | Omis (nu se aplică redirect) |

## 9. Fișiere statice

| Cale | Status | Răspuns |
|------|--------|---------|
| `/static/style.css` | 200 | CSS (Tailwind v4, ~21KB) |
| `/static/nonexistent.css` | 404 | Not Found |

---

## 10. Stripe Webhook

### `POST /stripe/webhook`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Payload valid + `checkout.session.completed` | 200 | `"OK"` |
| Payload valid + alt event (ignored) | 200 | `"Event ignored"` |
| Payload valid + `order_id` lipsă din metadata | 200 | `"No order_id"` |
| Payload deja procesat (idempotent) | 200 | `"Already processed"` |
| Fără header `stripe-signature` | 401 | `"Missing signature"` |
| Semnătură invalidă (HMAC greșit) | 401 | `"Invalid signature"` |
| JSON invalid | 400 | `"Invalid JSON"` |
| `STRIPE_WEBHOOK_SECRET` nesetat (dev mode) | N/A | Loghează warning, acceptă fără verificare |
| DB update eșuează | 500 | `"DB error"` |

---

## 11. Securitate — middleware și protecții

### 🔒 CSRF (Origin/Referer verification)
| Caz | Status | Răspuns |
|-----|--------|---------|
| POST cu `Origin: http://localhost:3001` | 200/302 | Permis |
| POST cu `Referer: http://localhost:3001/login` | 200/302 | Permis |
| POST fără `Origin` și fără `Referer` | 403 | `"CSRF check failed"` |
| POST cu `Origin: https://evil.com` | 403 | `"CSRF check failed"` |
| GET/PUT/DELETE/PATCH | N/A | Verificare CSRF NU se aplică |
| `SITE_URL` env var | N/A | Folosit ca referință pentru validare |
| `://localhost` în Origin/Referer | 200/302 | Permis (și pentru develop) |

### 🚦 Rate Limiting (login + signup)
| Caz | Status | Răspuns |
|-----|--------|---------|
| Primele 10 requesturi/minut la login | 200/302 | Normal |
| Al 11-lea request în aceeași minut | 302 | Redirect la `/login?error=Prea multe încercări...` |
| Primele 10 requesturi/minut la signup | 200/302 | Normal |
| Al 11-lea request la signup | 302 | Redirect la `/signup?error=Prea multe încercări...` |
| Reset după 60s | 200/302 | Contorul se resetează |
| Rate limiter per IP | N/A | Fiecare IP are contor propriu |

### 🔐 Account Lockout (login)
| Caz | Status | Răspuns |
|-----|--------|---------|
| 1-4 încercări eșuate login | 302 | `?error=Invalid credentials` |
| A 5-a încercare eșuată | 302 | `?error=Cont blocat temporar. Încearcă din nou peste 15 minute.` |
| Login reușit după așteptare | 302 | Normal (blocajul se șterge) |
| Login reușit înainte de blocaj | 302 | Normal + curăță contorul |
| Cheie de lockout | `ip:email` | IP diferit = contor diferit pentru același email |

---

## 12. Cont utilizator

### `POST /account/delete`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Neautentificat | 401 | `"Neautentificat"` |
| Autentificat | 302 | Șterge contul + redirect la `/` |

### `GET /account/export`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Neautentificat | 401 | `"Neautentificat"` |
| Autentificat | 200 | JSON cu datele utilizatorului |

---

## 13. Pagini de politici

### `GET /privacy`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără parametri | 200 | Pagină politică confidențialitate |

### `GET /security`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără parametri | 200 | Pagină politică securitate |

### `GET /.well-known/security.txt`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără parametri | 200 | `security.txt` (text simplu) |

---

## 14. Comportament POST /checkout (detaliat)

### `POST /checkout` — câmpuri validate

| Câmp | Obligatoriu | Validare |
|------|-------------|----------|
| `session_id` | Da | Alfanumeric + cratimă |
| `shipping_name` | Da | Minim 2 caractere |
| `shipping_address` | Da | Minim 5 caractere |
| `shipping_phone` | Da | 10 cifre (RO) |
| `guest_email` | Nu | Format email valid |
| `notes` | Nu | String, fără restricții |

Cazuri adiționale:
| Caz | Status | Răspuns |
|-----|--------|---------|
| Formular complet + coș valid | 302 | Redirect la Stripe checkout URL |
| Formular complet + coș gol | 302 | `?error=Coșul e gol` la `/cart` |
| Câmp obligatoriu lipsă | 302 | `?error=Cîmpul '...' lipsește` la `/checkout` |
| Stripe eșuează | 302 | Redirect la `/orders` |
| Neautentificat + email necompletat | 302 | `?error=...` |
| Autentificat | N/A | `user_id` se asociază automat comenzii |

---

## 15. Comportament POST /order/{id}/pay (detaliat)

| Caz | Status | Răspuns |
|-----|--------|---------|
| Neautentificat (fără token) | 302 | `?error=Trebuie să fii autentificat` la `/login` |
| Token invalid | 302 | `?error=Token invalid` la `/login` |
| Comanda nu există | 302 | `?error=Comanda nu există` la `/orders` |
| Comanda altui utilizator (IDOR) | 302 | `?error=Nu e comanda ta` la `/orders` |
| Comanda deja plătită | 302 | `?error=Deja plătită` la `/orders` |
| Valid + Stripe OK | 302 | Redirect la Stripe checkout URL |
| Valid + Stripe eșuează | 302 | Redirect la `/orders` |
| `user_id` null (guest) | 302 | Verify_ownership eșuează → `?error=Nu e comanda ta` |

---

## 16. Comportamente POST /admin (detaliat)

### `POST /admin/order/{id}/status`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Neautentificat | 200 | Redirect HTML (meta refresh) la login |
| Token valid + admin + status valid | 302 | Redirect + status actualizat |
| Token valid + admin + status invalid | 302 | `?error=Tranziție invalidă...` |
| Comandă inexistentă | 302 | `?error=Comanda negăsită` |
| Status `confirmed` fără plată | 302 | `?error=Comanda nu poate fi confirmată...` |
| Status `shipped` fără plată | 302 | `?error=Comanda nu poate fi expediată...` |
| Status `delivered` fără `shipped` | 302 | `?error=Tranziție invalidă...` |
| GET (method not allowed) | 405 | `405 Method Not Allowed` |

### `POST /admin/migrate-orders`
| Caz | Status | Răspuns |
|-----|--------|---------|
| Fără token | 401 | `"Admin: token lipsă"` |
| Token invalid | 401 | `"Admin: token invalid"` |
| Token valid + nu e admin | 403 | `"Admin: acces interzis"` |
| Token valid + admin | 200 | JSON `{"migrated": N}` |
| GET (method not allowed) | 405 | `405 Method Not Allowed` |

---

## 17. Metode HTTP nesesuportate (405)

| Cale | Method | Status | Răspuns |
|------|--------|--------|---------|
| `/cart/add` | GET | 405 | `405 Method Not Allowed` |
| `/cart/remove` | GET | 405 | `405 Method Not Allowed` |
| `/order/{id}/pay` | GET | 405 | `405 Method Not Allowed` |
| `/stripe/webhook` | GET | 405 | `405 Method Not Allowed` |
| `/admin/order/{id}/status` | GET | 405 | `405 Method Not Allowed` |
| `/admin/product/{slug}/delete` | GET | 405 | `405 Method Not Allowed` |
| `/admin/migrate-orders` | GET | 405 | `405 Method Not Allowed` |
| `/logout` | PUT/PATCH/DELETE | 405 | `405 Method Not Allowed` |

---

## 18. Headere de răspuns (toate rutele)

| Header | Prezent | Detalii |
|--------|---------|---------|
| `Content-Type` | Da | `text/html; charset=utf-8` sau `application/json` |
| `Content-Security-Policy` | Da | `default-src 'self'; script-src 'self'; ...` |
| `X-Content-Type-Options` | Da | `nosniff` |
| `X-Frame-Options` | Da | `DENY` |
| `Set-Cookie` | La login/signup | `token=...; HttpOnly; Secure; Path=/; SameSite=Lax` |
| `Set-Cookie` | La prima adăugare coș | `session_id=...; Path=/` |
| `Set-Cookie` | La logout | `token=; Max-Age=0` (șterge) |
| `Location` | La 302/301 | URL-ul de redirect |
