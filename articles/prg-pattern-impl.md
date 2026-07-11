# PRG (Post-Redirect-Get) Pattern

## Implementarea corectă în Rust+Axum fără JavaScript

### Problema

Un formular HTML standard cu `method="POST"` care întoarce HTML direct:

```html
<form method="POST" action="/checkout">
  <input name="email">
  <button type="submit">Plasează comanda</button>
</form>
```

```rust
async fn checkout_handler(body: String) -> Html<String> {
    let order = place_order(body).await;
    Html(format!("<h1>Comanda {} a fost plasată</h1>", order.id))
}
```

**Problema:** la refresh (F5), browserul întreabă "Trimite din nou datele formularului?" — și dacă utilizatorul confirmă, plasează o comandă DUBLUĂ.

### Soluția: PRG

PRG = Post → Redirect → Get.

Serverul NU întoarce HTML direct la POST. În schimb, întoarce un **redirect 302**:

```rust
async fn checkout_handler(body: String) -> Response {
    let order = place_order(body).await;
    // 302 → browserul face GET la /success
    (StatusCode::FOUND, [("Location", format!("/success?id={}", order.id))]).into_response()
}
```

Browserul primește 302, face automat un GET la `/success?id=...`. Acum F5 reîncarcă pagina GET, care e sigură și idempotentă.

### Implementarea în shop-mvp

#### Login handler (PRG cu cookie)

```rust
pub async fn login_handler(
    State(s): State<AuthState>,
    body: String,
) -> Response {
    let redirect = extract_redirect(&body);
    match auth_login(&s, &body).await {
        Ok((r, _)) => {
            // Set-Cookie + 302 — browserul salvează cookie-ul, APOI urmează redirect-ul
            let resp = (StatusCode::FOUND, [("Location", &redirect)]).into_response();
            resp.headers_mut().insert(SET_COOKIE, cookie);
            resp
        }
        Err(e) => {
            // Redirect ÎNAPOI la login cu eroare (PRG și pentru erori)
            (StatusCode::FOUND, [("Location", format!("/login?error={}&redirect={}", e, redirect))]).into_response()
        }
    }
}
```

**Detaliu important:** Set-Cookie + 302 în același răspuns. Browserul procesează ambele înainte să navigheze. Dacă ai face redirect prin script:

```html
<!-- GREȘIT: browserul execută scriptul DUPĂ ce cookie-ul e setat? Depinde de browser -->
<script>window.location.href = '/dashboard';</script>
```

`<script>` redirect vs 302 e o diferență fină dar critică. 302 e atomic: browserul salvează cookie-ul și navighează ca parte a aceleiași operații. Script redirect rulează asincron și comportamentul diferă între Chrome și Firefox.

#### Cart add (PRG cu error_back)

```rust
async fn cart_add(
    State(s): State<CartState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let form = parse_body::<AddItemForm>(&body).ok()?;
    let product = s.products.get_by_slug(&form.product_slug).await.ok()?;
    
    // Succes: redirect la pagina anterioară
    let referer = headers.get("referer").and_then(|v| v.to_str().ok()).unwrap_or("/products");
    let dest = format!("{}?success=Adăugat în coș", referer);
    // Notă: PRG adevărat ar separa POST de GET, dar redirect_back păstrează UX-ul
    (StatusCode::FOUND, [("Location", dest)]).into_response()
}
```

#### Checkout gol (PRG cu eroare)

```rust
// Coșul e gol → redirect la /cart cu eroare
if cart.items.is_empty() {
    return error_redirect(&format!("{}/cart", bp), "Coșul e gol");
}
```

Browserul primește 302 → face GET la `/cart?error=Coșul%20e%20gol` → pagina de cart arată eroarea.

### Gestionarea erorilor PRG

Erorile se întorc prin `?error=` în URL:

| Situație | Acțiune |
|----------|---------|
| Login eșuat | `302 /login?error=Email+invalid` |
| Signup eșuat | `302 /signup?error=Parola+prea+s curta` |
| Coș gol | `302 /cart?error=Coșul+e+gol` |
| Produs negăsit | `302 /products?error=Produs+negăsit` |
| Comanda nu există | `302 /orders?error=Comanda+nu+există` |

Template-ul citește `error` din URL și afișează:

```html
{% if error %}
<div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
    ❌ {{ error }}
    <a href="..." class="ml-4 text-red-800 underline">← înapoi</a>
</div>
{% endif %}
```

### Redirect-ul invizibil (script redirect)

Când handler-ul verifică autentificarea, nu poate returna 302 (pentru că deja a început să scrie body-ul). În acest caz, folosim un script redirect:

```rust
fn render_admin_redirect() -> Html<String> {
    Html(format!(r#"
        <!DOCTYPE html>
        <html><body>
        <script>window.location.replace('/login?redirect=/admin');</script>
        </body></html>
    "#))
}
```

**DEOARE** acest redirect NU e HTTP 302, ci o pagină HTML care execută JavaScript. Consecințe:
- Network tab arată 200 OK, nu 302
- Browserul pierde contextul (referer, history)
- E invizibil în instrumentele de rețea

**Soluția corectă:** refactorizează handler-ele să returneze `Response` (nu `Result<Html>`) și să poată returna 302 în orice moment.

### Pattern-ul complet

```
Browser                    Server
  │                          │
  │── POST /login ──────────►│
  │       (email, password)  │
  │                          │── verify credentials
  │                          │── set cookie
  │◄── 302 /dashboard ──────│
  │       (Set-Cookie)       │
  │                          │
  │── GET /dashboard ───────►│
  │       (Cookie: token=…)  │
  │                          │── verify token
  │                          │── render dashboard
  │◄── 200 HTML ────────────│
```

Fiecare pas e atomic și independent. F5 pe `/dashboard` → re-execută GET → nu dublează nimic.

### De ce să nu folosești HTMX sau fetch pentru asta

HTMX (prin `hx-post`) face un request AJAX, apoi INSEREAZĂ răspunsul în DOM. Asta înseamnă:
- URL-ul din bara de adrese NU se schimbă (fără `hx-push-url`)
- Bookmark-ul nu salvează starea
- Referer-ul e pagina anterioară, nu acțiunea
- Back button duce la starea înainte de acțiune, nu la acțiune

PRG cu formular nativ + 302:
- URL-ul se actualizează
- Bookmark-ul funcționează
- Back button e predictibil
- Funcționează fără JavaScript

### Testare

```bash
# Test PRG: POST ar trebui să întoarcă 302, nu 200
curl -s -o /dev/null -w "%{http_code}" -X POST \
  -d "email=test@test.org&password=parola123" \
  http://localhost:3001/login
# → 302 (FOUND)

# Test că 302 duce la destinația corectă
curl -s -o /dev/null -w "%{redirect_url}" \
  -X POST -d "email=test@test.org&password=parola123" \
  http://localhost:3001/login
# → http://localhost:3001/
```

### Meta

PRG nu e doar un pattern de prevenire a dublelor comenzi. E o arhitectură care face fiecare acțiune previzibilă, testabilă și sigură. În combinație cu server-side rendering (HN philosophy), elimina nevoia de stare client-side și face aplicația robustă împotriva unei clase întregi de bug-uri.
