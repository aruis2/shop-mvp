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
| `/static/style.css` | 200 | CSS |
| `/static/nonexistent.css` | 404 | Not Found |
