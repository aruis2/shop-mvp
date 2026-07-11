# Frontend Debugging: O Călătorie prin Bug-urile unui MVP

> *Cum am depanat redirect-urile, cookie-urile și stările într-un magazin online cu HTMX + Rust*

---

## Context

Un magazin online simplu: Rust (Axum) + Tera templates + HTMX 2.0.4. Fără framework JS, fără build step. Doar HTML trimis de server, cu HTMX pentru navigare parțială.

Sound simple? Ei bine, iată ce am învățat într-o singură sesiune de debugging.

---

## 1. Problema: Logout-ul te duce la home în loc să rămâi pe pagină

**Simptom:** Eram în coș, am dat logout, m-am trezit la pagina principală.

**Cauză:** Handler-ul de logout (GET `/logout`) returna un meta refresh la `/`. Folosea `window.location.href = '/'` hardcodat.

**"Fix" inițial:** Am adăugat parsing din header-ul `HX-Current-URL` (trimis de HTMX) și `Referer`.

```rust
let current_url = headers.get("hx-current-url")
    .or_else(|| headers.get("referer"))
    .and_then(|v| v.to_str().ok())
    .map(|s| s.to_string());
```

**Bug real:** `extract_path_from_url` presupunea că URL-ul conține `://` (ex: `http://host/cart`). Când i-am dat un path simplu gen `/cart` (din `?redirect=`), returna `"/"`.

```rust
// VERSIUNE GREȘITĂ
fn extract_path_from_url(url: &str) -> String {
    if let Some(s) = url.find("://") {  // ← "/cart" nu are "://"!
        // ... extrage calea
    }
    "/".to_string()  // ← fallback la home!
}
```

**Fix:** Adăugat branch pentru path-uri simple (`/cart`, `/products?page=2`).

---

## 2. Problema: Adminul nu merge, deși ești logat

**Simptom:** Email-ul apare în nav, dar la click pe "Admin" te duce la login.

**Cauză:** Adminul încerca să citească token-ul din `localStorage.getItem('token')`. Dar `token` e în HttpOnly cookie (invizibil din JS), nu în localStorage.

```javascript
// COD VECHI (nu mai funcționează)
var t = localStorage.getItem('token');  // ← null! token-ul e în cookie
if (t) { window.location.replace('/admin?token=' + t); }
else { window.location.replace('/login'); }  // ← ajunge aici
```

**Fix:** Schimbat `render_admin_redirect` să redirecționeze direct la login. Serverul verifică cookie-ul direct în `verify_admin()`.

**Moment "aha":** `localStorage` și `cookie` sunt locuri diferite cu scopuri diferite. `cookie` e pentru server (trimis automat la fiecare request). `localStorage` e pentru client (doar JS îl citește). Token-ul JWT trebuie să fie în cookie (HttpOnly) ca să nu poată fi furat de XSS.

---

## 3. Problema: Checkout-ul crapă cu "Coșul e gol" deși coșul are produse

**Simptom:** Pagina coșului arată produse, dar la click pe "Finalizează comandă" primești eroare 400.

**Cauză:** `checkout_page` citea `session_id` doar din query param și header `X-Session-Id`. `cart_page` citea și din cookie `session_id`. Când navighezi direct la `/checkout` (nu prin HTMX), header-ul `X-Session-Id` lipsește.

```rust
// checkout_page (VECHI) — nu citea din cookie
let sid = q.session_id.clone().or_else(|| {
    headers.get("x-session-id")  // ← absent la navigare directă!
}).unwrap_or_else(|| "anon".to_string());

// cart_page — citea și din cookie ✓
let session_id = q.session_id.as_deref()
    .or_else(|| headers.get("x-session-id"))
    .or_else(|| headers.get("cookie")...)  // ← fallback la cookie
    .unwrap_or("anon");
```

**Fix:** Adăugat citirea din cookie și în `checkout_page`.

**Moment "aha":** HTMX adaugă header-e automate (`X-Session-Id` prin `htmx:configRequest`), dar navigarea directă (refresh, link fără `hx-get`) nu le are. Cookie-urile sunt singurul mecanism care funcționează în ambele moduri.

---

## 4. Problema: După login, nu apare email-ul și logout-ul în nav

**Simptom:** Te loghezi cu succes, dar în nav nu apare email-ul tău. Pagina pare că nu știe că ești logat.

**Cauză:** Am schimbat `htmx_auth_script` să returneze `HX-Redirect` header în loc de `<script>window.location.href=...`. HTMX procesează `HX-Redirect` INAINTE să execute script-ul din corpul răspunsului. Rezultat: `localStorage.setItem('user', ...)` nu se execută niciodată pentru că browserul navighează înainte.

```rust
// VERSIUNE GREȘITĂ
fn htmx_auth_script(token, user, redirect) {
    // Scriptul care salvează user-ul
    let html = "<script>localStorage.setItem('user', ...)</script>";
    // HX-Redirect face navigarea INAINTE să se execute script-ul!
    resp.headers_mut().insert("hx-redirect", redirect);
}
```

**Fix:** Revenit la varianta cu `<script>` care face ambele operații: mai întâi `localStorage.setItem`, apoi `window.location.href`.

**Lecție:** `HX-Redirect` e util pentru redirect-uri simple, dar când ai nevoie să execuți cod înainte de navigare (gen salvare în localStorage), trebuie să folosești script + `window.location.href`.

---

## 5. Problema: Login/Signup redirect ignore `?redirect=` din URL

**Simptom:** Deși link-ul de login din nav are `?redirect=/products`, după login ajungi la home.

**Cauză dublă:**

1. Link-ul din nav e creat de `shop.js` care are `href="/login?redirect=..."` DAR **nu are** `hx-get`. Link-urile din `base.html` (Shop, Produse, Coș) au `hx-get` hardcodat, dar login-ul e adăugat dinamic. Deci e doar un `<a>` simplu — navigare completă. Asta înseamnă că `?redirect=` ajunge în URL. Bun.

2. Problema reală: Când ești pe `/products` și dai click "Autentificare", ajungi pe `/login?redirect=%2Fproducts`. Pagina de login se încarcă, formularul are `<input type="hidden" name="redirect" value="/products">`. Completezi login, dai submit. **Dar** formularul e trimis via HTMX (`hx-post`). HTMX trimite POST `/login` cu `Referer: http://localhost:3001/login` (pagina de login, nu products!). Serverul citește `redirect` din formular... dar `extract_redirect(body)` poate e gol? Sau... `auth_login` are un fallback pe `referer` care e URL-ul paginii de login, nu al paginii originale.

**Debugging:** Am adăugat `tracing::warn!` în toate funcțiile implicate. Am văzut în loguri:

```
login_page: q.redirect=None referer=Some("http://localhost:3001/login") computed_redirect=Some("/login")
auth_redirect: redirect='/login' -> going to /login
```

Ce s-a întâmplat: utilizatorul a navigat la `/login` fără `?redirect=` (pentru că shop.js era în cache cu versiunea veche). Serverul a citit `Referer: http://localhost:3001/login` (pagina de login însăși, de la o redirecționare anterioară). `extract_path_from_referer` a returnat `/login`. Și așa s-a intrat într-un loop: login → redirect la login → login → redirect la login.

**Fix:** Trei straturi de fallback:
1. `?redirect=` în URL (client-side)
2. `extract_path_from_referer` la randarea paginii de login (server-side)
3. `Referer` la submit-ul formularului (server-side, doar ca ultimă soluție)

Plus: în `auth_login` și `auth_signup`, fallback-ul pe `referer` folosește URL-ul complet (`http://localhost:3001/login`), nu calea. Asta merge la `HX-Redirect` (acceptă URL complet) dar script-ul cu `window.location.href` merge și el.

---

## 6. Problema: După ce te loghezi, dacă dai refresh pe `/login`, rămâi blocat

**Simptom:** Ești logat, dai din greșeală `/login`, și pagina de login se încarcă în loc să te redirecționeze.

**Fix:** Serverul verifică cookie-ul în `login_page` și `signup_page`:

```rust
if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
    if let Some(token) = crate::cookie::get_cookie(cookie, "token") {
        if s.auth.verify_token(token).await.is_ok() {
            return Ok(redirect_html(&dest));
        }
    }
}
```

---

## 7. Lecții Învățate

### Despre Cache

Browserul cache-uiește agresiv fișierele statice (`shop.js`). Am petrecut o oră debugging o problemă care de fapt era doar cache. Soluția: `?v=N` în URL sau instrucțiunea "fă un Ctrl+F5".

**Moment "aha":** Când utilizatorul spune "nu merge" și ție îți merge în Playwright, e cache. Sau e un server diferit. Sau e o extensie de browser. Sau e configurația de rețea. Debugging-ul remote e greu.

### Despre HTMX

| Mecanism | Când se execută | Bun pentru |
|----------|----------------|------------|
| `HX-Redirect` header | Imediat, înainte de swap | Redirect-uri simple |
| `<script>` în response | După swap | Operații înainte de navigare (localStorage) |
| `hx-select` | La swap | Extragerea unei părți din response |
| `htmx:configRequest` | La fiecare request | Adăugat header-e dinamice |

### Despre Cookie vs localStorage

| Caracteristică | Cookie (HttpOnly) | localStorage |
|---------------|-------------------|--------------|
| Trimis automat la server | Da | Nu |
| Accesibil din JS | Nu | Da |
| Bun pentru | Token JWT, session_id persistent | User info (email, rol) |
| Expirare | Max-Age | Manual (până ștergi) |

### Despre Redirect

**The Golden Rule of Redirect:** Serverul trebuie să știe în orice moment de unde a venit utilizatorul și încotro se duce. Nu lăsa asta doar în seama clientului — browserul poate avea cache, extensii pot bloca Referer-ul, HTMX poate sări peste headere.

Am învățat să am **trei straturi** de redirect:
1. `?redirect=` în URL (client)
2. `Referer` la randarea paginii (server)
3. Cookie persistent (server)

---

## Concluzie

Cel mai simplu stack (Rust + HTML + HTMX) poate avea bug-uri la fel de subtile ca un SPA cu React. Diferența e că aici **fiecare request e un request HTTP normal** — poți da refresh, poți bookmark-ui, poți testa cu curl.

Framework-urile JS ascund complexitatea (routing, state management, cache). HTMX nu ascunde nimic — e doar HTTP cu un pic de automare. Când ceva nu merge, e pentru că nu ai înțeles cum funcționează HTTP, nu pentru că un abstractions layer ți-a ascuns bug-ul.

Și da, cache-ul browserului e inamicul tău numărul 1.
