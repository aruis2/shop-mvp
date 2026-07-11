# Filosofia Hacker News în Arhitectura Web Modernă

## De ce o pagină din 2007 e mai robustă decât majoritatea Single-Page Applications

### Context

Hacker News (HN) a fost lansat în 2007 de Paul Graham, scris în Arc (un dialect Lisp). Serverul randează HTML pur, fără framework-uri client-side, fără JavaScript în producție. Pagina se încarcă în ~200ms pe conexiuni 3G. Și funcționează impecabil de 18 ani.

Contrastul e șocant: în 2026, majoritatea aplicațiilor web moderne au bundle-uri JS de 2-5MB, timeout-uri la API-uri, waterfall-uri de request-uri, și o durată medie de viață a unui framework de 18 LUNI, nu ani.

### Principiile HN

#### 1. Conținutul este text, nu aplicație

HN servește HTML. Nu "aplicații", nu componente reactive, nu virtual DOM. Serverul primește un request și returnează text. Browserul îl afișează. Gata.

În practică, asta înseamnă:
- Zero JS în primul paint
- Fără waterfall: un singur request → răspuns complet
- Funcționează cu orice browser din ultimii 20 de ani
- Accesibil din linia de comandă (`curl`, `wget`, `lynx`)

```bash
# Poți citi HN din terminal
curl https://news.ycombinator.com | grep "title"
```

#### 2. Fiecare cerere e atomică

Nu există stare client-side. Nu există "rehydratare". Serverul nu știe și nu îi pasă ce aveai înainte. Fiecare request produce un răspuns complet și determinist.

Asta elimină categorii întregi de bug-uri:
- Stare inconsistentă client/server: imposibil
- Race condition-uri între API-uri: imposibil
- Memory leaks client-side: imposibil
- Problema "dar pe calculatorul meu mergea": mult mai rară

#### 3. Simplitatea e un feature de securitate

HN nu are nevoie de:
- CORS configuration
- CSRF tokens (formularele sunt simple POST-uri)
- Content Security Policy complicată (niciun script inline)
- JWT în localStorage
- XSS protecție (nu există template-uri client-side)

Fiecare linie de cod care NU există e o linie de cod care NU poate avea bug-uri.

#### 4. Performanță predictibilă

Costul unui request HN e strict:
```
t_request = t_network + t_server + t_network
```

Nu există variabile ascunse:
- Nu descarci un bundle JS după HTML
- Nu aștepți API-uri după prima randare
- Nu aștepți fonturi web (HN folosește fonturi de sistem)
- Nu renderuiești de mai multe ori (hydration)

### Implementare în shop-mvp

Aplicând această filosofie în Rust+Axum:

```rust
// Fiecare handler randează HTML complet, fără apeluri API client-side
async fn products_page(
    State(s): State<ProductState>,
    Query(q): Query<ProductsQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    let (products, total) = s.products.get_products(None, page, PER_PAGE).await?;
    let ctx = build_context(products, total, page);
    render_or_err(&s.renderer, "products.html", &ctx, &bp)
}
```

Ce nu facem:
- ❌ Returnăm JSON pe care clientul să-l randeze
- ❌ Returnăm HTML parțial pe care clientul să-l îmbine
- ❌ Returnăm date și lăsăm clientul să decidă

Ce facem:
- ✅ Serverul produce HTML complet și final
- ✅ Fiecare request → răspuns independent
- ✅ Starea e în URL (query params), nu în JavaScript

### Când nu se aplică

HN philosophy nu e potrivită pentru:
- Aplicații real-time (chat, colaborare)
- Interfețe cu drag-and-drop complex
- Dashboard-uri cu update-uri în timp real

Pentru un magazin online (catalog, coș, checkout, comenzi), este soluția ideală: fiecare acțiune e o navigare, nu o tranziție de stare.

### Meta

În 2026, "modern web" înseamnă de obicei framework-uri client-side masive. HN ne reamintește că server-side rendering pur nu e o limitare, ci o virtute. Textul e rapid. Textul e robust. Textul durează 18 ani.

Asta nu înseamnă să nu folosești JavaScript deloc (HN are script pentru votare). Înseamnă să întrebi: **"Chiar am nevoie de JavaScript pentru asta?"** — și de cele mai multe ori, răspunsul e nu.
