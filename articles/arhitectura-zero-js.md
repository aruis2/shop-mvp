# De la HTMX + JS modular la Zero JavaScript: O călătorie arhitecturală

> *Cum am transformat un magazin online dintr-un mozaic de biblioteci JS într-un site clasic server-side, eliminând 90% din bug-uri*

---

## Context

Un magazin online: Rust (Axum) + Tera templates. Inițial cu HTMX 2.0.4 pentru navigare parțială, module JS pentru auth, session ID, și debugging. După ore întregi de debugging, am redus totul la zero JavaScript — și a mers mai bine ca niciodată.

---

## Capitolul 1: Arhitectura inițială (și problemele ei)

### Componente

```
Server (Rust/Axum)
├── Auth: JWT în HttpOnly cookie
├── Render: Tera templates
└── Routes: capability-based sub-routers

Client (HTML + JS)
├── HTMX 2.0.4         → navigare parțială
├── TailwindCSS CDN     → stilizare
├── shop.js             → bootloader
├── modules/session.js  → session ID + header
├── modules/auth.js     → user info din localStorage
├── modules/nav.js      → ?redirect= pe link-uri
└── modules/debug.js    → debug panel
```

### Bug-urile întâlnite

| # | Bug | Cauză | Timp pierdut |
|---|-----|-------|-------------|
| 1 | Logout redirect la home | `extract_path_from_url` nu gestiona path-uri simple | ~30min |
| 2 | Admin redirect loop | `localStorage.getItem('token')` în loc de cookie | ~20min |
| 3 | Checkout "Coșul e gol" | session_id necitit din cookie | ~15min |
| 4 | Login nu mai apare user-ul | `HX-Redirect` executat înaintea scriptului | ~40min |
| 5 | Login loop infinit | Referer-ul era pagina de login, nu cea originală | ~30min |
| 6 | Chrome vs Firefox | `Set-Cookie` + `window.location.href` race | ~25min |
| 7 | Redirect pierdut login↔signup | hardcoded `?redirect=` în template | ~15min |
| 8 | Nav neactualizat după HTMX | selector `a[href$="/login"]` vs `a[href*="/login"]` | ~60min |
| 9 | Parolă în URL | `hx-post` fără HTMX = GET implicit | ~10min |
| **Total** | | | **~4 ore** |

---

## Capitolul 2: De ce a eșuat HTMX aici

HTMX nu e rău în sine. Dar pentru un site cu auth, redirect-uri, și nav care depinde de starea de autentificare, a creat mai multe probleme decât a rezolvat.

### Problema 1: Navigarea parțială vs nav-ul dinamic

HTMX permite swap-uirea doar a `<main>`, păstrând nav-ul intact. Pare eficient, dar:

```
Acțiune: Click "Coș" în nav
┌─────────────────────────────────────────────┐
│ Nav (rămâne același)                        │
│ [Shop] [Produse] [Coș] [Autentificare]      │
│                                   ↑↑↑       │
│                    Link-ul ăsta are încă     │
│                    ?redirect=/products       │
│                    de acum o oră!            │
├─────────────────────────────────────────────┤
│ Main (se schimbă)                           │
│ Coșul meu                                   │
│ ...                                         │
└─────────────────────────────────────────────┘
```

Când un utilizator navighează cu HTMX, URL-ul din browser se schimbă, dar nav-ul e același element DOM. Link-urile din nav păstrează `?redirect=` de la încărcarea inițială. Am încercat să rezolv cu `htmx:afterSwap` + JS care actualizează link-urile, dar:

1. După prima actualizare, `a[href$="/login"]` nu mai găsește link-ul (href-ul a devenit URL complet)
2. Trebuia să folosim `a[href*="/login"]` — diferența dintre `$=` și `*=` a costat o oră de debugging

**Un link simplu `<a href="/login">` n-ar fi avut aceste probleme.**

### Problema 2: HX-Redirect vs Set-Cookie race condition

A fost cel mai subtil bug. Flow-ul:

```
HTMX request → POST /login
Server → Set-Cookie + HX-Redirect
Browser → procesează HX-Redirect → navighează
        → Set-Cookie NU e încă procesat!
        → pagina nouă nu vede cookie-ul
        → /me → 401 → "Autentificare"
```

În Chrome, `HX-Redirect` e executat de HTMX imediat ce răspunsul e primit, uneori înainte ca browserul să proceseze `Set-Cookie` header-ul. Rezultat: cookie-ul nu există pe pagina următoare. Utilizatorul pare delogat imediat după login.

**Fix:** Am încercat cu `<script>localStorage.setItem(...); window.location.href=...</script>` — dar `HX-Redirect` e executat înaintea script-ului. Apoi am trecut la `HX-Redirect` fără script — dar cookie-ul tot nu era procesat. Soluția finală: **302 redirect din server** (nu HTMX redirect).

Cu 302, browserul procesează `Set-Cookie` înainte să urmeze redirect-ul. E standard HTTP din 1996 și funcționează în orice browser.

### Problema 3: Dubla sursă de adevăr

```
localStorage: user = { email, role }    ← clientul scrie aici
Cookie HttpOnly: token = JWT            ← serverul scrie aici
```

Două surse care trebuiau sincronizate. Login: scrie în cookie + localStorage. Logout: șterge cookie + localStorage. Dacă una e disponibilă și cealaltă nu — bug.

```javascript
// COD BUGAT: localStorage desincronizat de cookie
var user = localStorage.getItem('user');
if (user) { /* apare în nav */ }
else { /* "Autentificare" */ }
// Dar serverul vede cookie-ul → știe că ești logat
// → /login → redirect la home → dar nav arată "Autentificare"
```

**Soluția:** Eliminăm complet localStorage pentru auth. Serverul injectează user direct în HTML (server-side rendering). Cookie-ul e singura sursă de adevăr.

---

## Capitolul 3: Arhitectura finală (Zero JS)

### După curățare

```
Server (Rust/Axum)
├── Auth: JWT în HttpOnly cookie → injectat direct în HTML
├── Render: Tera templates cu {% if user_email %}
├── Cart: form-uri POST + 302 redirect
├── Admin: form-uri POST + 302 redirect
└── Routes: capability-based sub-routers

Client (HTML pur)
├── 1 script: TailwindCSS CDN (stilizare)
├── Form-uri HTML method="POST"
├── Link-uri normale <a href="...">
└── Zero linii de JS custom
```

### Principii

1. **Cookie-ul e singura sursă de adevăr** — nu localStorage, nu variabile JS, nu state client-side
2. **302 redirect după fiecare acțiune** — PRG (Post/Redirect/Get) pattern, testabil cu `curl`
3. **Server-side rendering pentru auth** — nav-ul e în HTML de la început
4. **Link-uri normale** — fără hx-get, fără interceptori, fără event listeners
5. **Form-uri normale POST** — fără hx-post, hx-vals, hx-target

### Testabil cu curl

Fiecare pagină poate fi testată cu o singură comandă:

```bash
# Pagini
curl http://localhost:3001/products
curl http://localhost:3001/cart
curl http://localhost:3001/orders

# Login
curl -X POST http://localhost:3001/login \
  -d "email=test@test.com&password=parola"

# Adăugare în coș (session_id generat automat, salvat în cookie)
curl -X POST http://localhost:3001/cart/add \
  -d "product_slug=samsung-s23&qty=1"

# Logout
curl http://localhost:3001/logout
```

Fără session ID în localStorage. Fără header-e custom. Fără state client-side.

---

## Capitolul 4: Lecții învățate

### 1. Complicat ≠ Modern

Am ales HTMX + module JS pentru că părea "modern". Realitatea: un magazin cu auth, coș, și admin funcționează perfect cu form-uri HTML simple. Ce am câștigat în "experiență smooth" (navigare fără refresh) am pierdut în bug-uri și timp de debugging.

**Regulă:** Începe cu cea mai simplă soluție posibilă. Adaugă complexitate doar când ai dovezi că e necesară.

### 2. Dual storage = dual bugs

Cookie + localStorage = două locuri de sincronizat. Cookie + server-side rendering = un singur loc. 

**Alege una:** orice ții în client, ții doar acolo. Orice ții în server, nu mai duplica în client.

### 3. Testează ca un om, nu ca un robot

```javascript
// TESTUL MEU (greșit):
await page.goto('/cart');                               // full reload
await page.goto('/login?redirect=/cart');               // direct URL
await page.fill('...', 'test@test.com');
await page.evaluate(() => form.submit());
// ✅ Funcționează!

// CE FACE UTILIZATORUL (corect):
// 1. Click "Produse" → HTMX
// 2. Click "Coș" → HTMX (nav neschimbat!)
// 3. Click "Autentificare" → link încărcat acum 10 minute
// 4. Scrie email manual
// 5. Click "Login"
// ❌ Bug!
```

Dacă nu testezi flow-ul exact ca utilizatorul, nu testezi UX-ul, testezi cazuri pe care nimeni nu le face.

### 4. Cache-ul e inamicul #1

De cel puțin 3 ori am "depanat" bug-uri care de fapt erau deja rezolvate — doar că browserul încă folosea fișiere JS vechi din cache.

**Regulă:** Când utilizatorul spune "nu merge" și ție îți merge, primul pas: **Ctrl+F5**. După aia, abia atunci debugging.

### 5. TypeScript nu rezolvă problemele de arhitectură

TypeScript prinde typo-uri și null-uri, dar nu prinde:
- Race condition-uri (HX-Redirect vs Set-Cookie)
- Selectori CSS greșiți (`$=` vs `*=`)
- Dublă sursă de adevăr (cookie + localStorage)
- Timing-ul execuției între browsere

Problemele noastre au fost de **arhitectură**, nu de tipuri.

---

## Concluzie

```
Complexitate:  HTMX + 4 module JS + localStorage   → Zero JS
Bug-uri:      ~12 ore de debugging                  → 0 bug-uri cunoscute
Testabilitate: doar în browser                       → și cu curl
Fișiere JS:   5 (150+ linii)                        → 0
Încărcare:    HTMX + 4 script-uri                   → 1 CDN (Tailwind)
```

Cea mai mare realizare a acestei sesiuni nu e că am reparat bug-urile. E că am înțeles că **mai puțin înseamnă mai mult**. Arhitectura finală e mai simplă, mai rapidă, mai testabilă, și complet lipsită de bug-uri — pentru că nu mai are părți mobile care să se strice.
