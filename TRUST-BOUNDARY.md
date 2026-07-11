# 🔐 Trust Boundary — Granița de încredere a aplicației

> Ce intră, ce iese, și în ce avem încredere.
> Arhitectura cu 3 fabrici + query validator + LEGO modules + capabilities.
> Actualizat: 2026-07-11

---

## Harta completă a graniței

```
                    ┌──────────────────────────────────────────────────────┐
                    │               NELIPSIT DE ÎNCREDERE                  │
                    │         (Outside World — necontrolat de noi)         │
                    │                                                      │
                    │  HTTP request · Browser · curl · Atacatori · Bots    │
                    │  Rețea externă · CDN · Cloud · Telefon (S22)         │
                    └──────────────────┬───────────────────────────────────┘
                                       │
                         body, query, headers: RAW String
                         ─── totul e neverificat, periculos ───
                                       │
                     ╔═════════════════╧══════════════════╗
                     ║        STRAJA 1: INPUT FACTORY      ║
                     ║  (ce intră — validare tip + format) ║
                     ╚═════════════════╤══════════════════╝
                                       │
              ┌──────────────────────┼──────────────────────┐
              │                       │                       │
         parser.rs               InputFactory            QueryValidator
         parse_form()            parse_email()           page()
         get_field()             parse_price()           uuid()
         parse_any_into()        parse_slug()            token()
                                 parse_qty()             session_id()
                                 parse_phone()
                                 parse_name()
                                 parse_address()
                                 parse_brand()
                                 parse_search()
                                 parse_session_id()
                                 etc. (17 metode)
              │                       │                       │
              └──────────────────────┼──────────────────────┘
                                       │
                     ╔═════════════════╧══════════════════╗
                     ║       STRAJA 2: LOGIC FACTORY       ║
                     ║  (reguli de business — e permis?)   ║
                     ╚═════════════════╤══════════════════╝
                                       │
               verify_ownership()     ← IDOR prevention
               verify_admin()         ← authorization
               verify_not_paid()      ← double payment
               verify_status_transition() ← state machine
               verify_stock_available() ← inventory
               verify_qty_in_range()
               verify_max_value()
               verify_found()
               verify_not_duplicate()
                                       │
                                       ▼
                     ╔════════════════════════════════════╗
                     ║      ZONA DE ÎNCREDERE (TRUSTED)    ║
                     ║  Date garantat valide + permise     ║
                     ╚═════════════════╤══════════════════╝
                                       │
              ┌──────────────────────┼──────────────────────┐
              │                       │                       │
           Handlere (flow)        LEGO modules            DB (queries)
           (orchestrează)         (capabilities)          (prin trait-uri)
                                  rust-cart               SELECT FOR UPDATE
                                  rust-auth               INSERT ON CONFLICT
                                  rust-orders             tranzacții
                                  rust-payment
                                  rust-products
              │                       │                       │
              └──────────────────────┼──────────────────────┘
                                       │
                     ╔═════════════════╧══════════════════╗
                     ║      STRAJA 3: OUTPUT FACTORY       ║
                     ║  (ce iese — sanitizare + siguranță) ║
                     ╚═════════════════╤══════════════════╝
                                       │
              html_encode()           ← XSS prevention
              safe_redirect_url()     ← open redirect prevention
              safe_error_msg()        ← error messages safe
              safe_header_value()     ← HTTP response splitting
              safe_cookie_value()     ← cookie injection
              sanitize_context()      ← Tera context walk
              make_context()          ← JSON → safe Context
                                       │
                                       ▼
                    ┌──────────────────────────────────────┐
                    │          IEȘIRE (spre exterior)        │
                    │                                       │
                    │  ← 200 HTML (Tera auto-escape +       │
                    │         OutputFactory sanitize)       │
                    │  ← 302 redirect (safe_redirect_url)   │
                    │  ← Set-Cookie (safe_cookie_value)     │
                    │  ← 4xx/5xx (safe_error_msg)           │
                    │  ← CSP headers (XSS, clickjacking)    │
                    └──────────────────────────────────────┘
```

---

## Cele 3 fabrici — responsabilități clare

| Fabrică | Ce verifică | Intrare | Ieșire | Teste |
|---------|------------|---------|--------|-------|
| **InputFactory** | Format + tip (sintaxă) | `&str`, `i32` | `Email`, `Price`, `Slug`, etc. | 17 metode |
| **LogicFactory** | Reguli business (semantică) | tipuri validate | `Result<(), LogicError>` | 23 de teste |
| **OutputFactory** | Sanitizare ieșire | `&str`, `serde_json::Value` | HTML safe, URL safe, header safe | 39 de teste |
| **QueryValidator** | Query params invalide | `Option<T>` | `T` valid + log warning | - |

## Ce e în afara graniței (NEÎNCREDERE)

| Componentă | De ce nu avem încredere |
|-----------|------------------------|
| **Browser** | Orice browser poate trimite orice — cookie-uri modificate, form-uri false, header-e false |
| **Rețea** | HTTP poate fi interceptat — de asta avem HTTPS + HSTS + CSP |
| **curl / API calls** | Oricine poate face request-uri — nu știm cine e |
| **S22 (telefon)** | E o mașină separată, rețea locală — dar tot nu controlăm ce rulează acolo |
| **CDN / Cloud** | Nu controlăm serverele intermediare |
| **DB (remote)** | Remote DB e prin SSH tunel — traficul e criptat, dar DB e pe altă mașină |

## Ce e la granița de intrare (InputFactory + parser.rs)

| Componentă | Ce face |
|-----------|---------|
| **`parser.rs`** | Parsează URL-encoded body + JSON în `FormField[]` — zero dependințe externe |
| **`parse_any_into()`** | Acceptă JSON sau form-urlencoded, unic punct de intrare pentru body |
| **`InputFactory::parse_*()`** | 17 metode care transformă `&str`/`i32` în tipuri sigure |
| **`QueryValidator`** | Validează query params (`page`, `uuid`, `token`, `session_id`) + header-e (`x-session-id`) — loghează valori invalide |
| **`cookie.rs`** | Citește/Scrie cookie-uri prin `safe_cookie_value()` |

### Ce body-uri trec prin InputFactory

| Handler | Cîmpuri | InputFactory |
|---------|---------|-------------|
| `cart_add` | `product_slug`, `qty` | `parse_slug()`, `parse_qty()` |
| `cart_remove` | `item_id` | `parse_any_into()` |
| `checkout` | `session_id`, `guest_email`, `shipping_name`, `shipping_address`, `shipping_phone`, `notes` | 6 metode |
| `admin_product_create` | `brand`, `name`, `slug`, `price_new`, `stock_count` | 5 metode |
| `admin_product_update` | `brand?`, `name?`, `slug?`, `price_new?`, `stock_count?` | 5 metode (opționale) |
| `admin_order_status` | `status` | `parse_any_into()` |
| `auth_signup` | `email`, `password` | `parse_email()` |
| `auth_login` | `email`, `password` | `parse_email()` |

### Ce query params + header-e trec prin validare

| Sursă | Validare | Ce face cînd e invalid |
|-------|----------|----------------------|
| `?page=` | `QueryValidator::page()` → ≥ 1 | Loghează warning, default=1 |
| `?token=` | Verificat de JWT (`auth.verify_token()`) | 401 Unauthorized |
| `?redirect=` | `OutputFactory::safe_redirect_url()` | Fallback la `/` |
| `?session_id=` | `QueryValidator::session_id()` → max 256 chars | Ignorat, fallback la cookie |
| `?order_id=` | `Uuid::parse_str()` | Ignorat (success page) |
| `?q=` | `InputFactory::parse_search()` → max 200 chars | 400 Bad Request |
| `?category=` | Serde parse i32 → Option | None → fără filtru |
| `?error=` | `OutputFactory::safe_error_msg()` | Sanitarizat la afișare |
| **Header: `x-session-id`** | `QueryValidator::session_id()` → max 256 chars | Ignorat, fallback la cookie |
| **Header: `Cookie`** | `cookie::get_cookie()` + `safe_cookie_value()` | Token invalid → 401 |
| **Header: `Authorization`** | `auth.verify_token()` (JWT) | 401 Unauthorized |
| **Header: `Referer`** | `OutputFactory::safe_redirect_url()` la ieșire | Fallback la `/` |
| **Header: `HX-Request`** | Doar verificare prezență (boolean) | - |
| **Header: `HX-Current-Url`** | `OutputFactory::safe_redirect_url()` la ieșire | Fallback la `/` |
| **Header: `X-Forwarded-For`** | Rate limiter (string, IP) | Rate limitat la 10/min

## Ce e la granița de ieșire (OutputFactory)

| Componentă | Ce face |
|-----------|---------|
| **`OutputFactory::html_encode()`** | Escape `& < > " '` — previne XSS |
| **`OutputFactory::safe_redirect_url()`** | Blochează `javascript:`, `data:`, `//` — previne open redirect |
| **`OutputFactory::safe_error_msg()`** | Elimină control chars, trunchiază la 200 |
| **`OutputFactory::safe_header_value()`** | Elimină newline-uri — previne HTTP response splitting |
| **`OutputFactory::safe_cookie_value()`** | Elimină caractere periculoase din cookie-uri |
| **`OutputFactory::sanitize_context()`** | Walk recursiv Tera Context — html_encode pe toate string-urile |
| **`OutputFactory::make_context()`** | Creează Context sanitizat din serde_json::Value |
| **`RenderService::render_json()`** | Aplică OutputFactory înainte de Tera — automat |
| **`cookie::set_cookie()`** | HttpOnly + safe_cookie_value — previne cookie injection |

## Ce e în zona de încredere (TRUSTED)

| Componentă | De ce avem încredere |
|-----------|---------------------|
| **Handlere** | Primesc doar tipuri sigure — InputFactory + LogicFactory au verificat |
| **LEGO modules** | Capability-based — fiecare handler are doar ce-i trebuie |
| **DB (local)** | Noi am scris datele validate — le citim în aceleași tipuri |
| **Templates (Tera)** | Auto-escape HTML + OutputFactory pre-sanitizare — XSS imposibil |
| **`state.rs`** | Axum capability — handler-ele nu pot accesa ce nu trebuie |
| **Logger** | Scrie în fișier local, append-only, fără date sensibile |

## Mecanisme de apărare în adîncime

```
Strat 1: InputFactory    ← parsează + validează tipul (sintaxă)
Strat 2: QueryValidator  ← loghează query params invalide
Strat 3: LogicFactory    ← verifică permisiunile (semantică)
Strat 4: LEGO modules    ← capability-based (nu poți ce nu ai)
Strat 5: tranzacții      ← SELECT FOR UPDATE, INSERT ON CONFLICT
Strat 6: OutputFactory   ← sanitizează tot ce iese
Strat 7: Tera auto-escape ← template-level XSS protection
Strat 8: CSP headers     ← script-src, frame-ancestors, object-src
```

## Regulile de aur

1. **Tot ce trece granița dinspre exterior spre interior** → `parse_any_into()` + `InputFactory::parse_*()`
2. **Orice query param de la utilizator** → `QueryValidator::*()` sau verificare explicită
3. **Orice decizie de business (e permis?)** → `LogicFactory::verify_*()`
4. **Tot ce iese din interior spre exterior** → `OutputFactory::safe_*()`
5. **DB e sursă de încredere doar pentru citire** — scrierea s-a făcut deja prin InputFactory
6. **Dacă nu e verificat la graniță, nu există în interior**
7. **Read-then-write operations** → `SELECT FOR UPDATE` în tranzacție
8. **Upsert-uri** → `INSERT ... ON CONFLICT DO UPDATE` (previne race conditions)

## Această graniță previne

| Atac | Prevenit de |
|------|------------|
| XSS (reflected + stored) | OutputFactory + Tera auto-escape + CSP |
| XSS (în context JavaScript) | OutputFactory::html_encode + CSP (script-src 'self') |
| Open redirect | OutputFactory::safe_redirect_url |
| HTTP response splitting | OutputFactory::safe_header_value |
| Cookie injection / fixation | OutputFactory::safe_cookie_value + HttpOnly |
| SQL injection | SQLx query_as (parametrized queries) |
| IDOR (broken object access) | LogicFactory::verify_ownership |
| Privilege escalation | LogicFactory::verify_admin + capability-based state |
| Payment double-charge | LogicFactory::verify_not_paid + idempotency cache |
| Invalid state transition | LogicFactory::verify_status_transition |
| Race condition (cart add) | INSERT ON CONFLICT + UNIQUE constraint |
| Lost update (product edit) | SELECT FOR UPDATE în tranzacție |
| Clickjacking | CSP frame-ancestors 'none' |
| Plugin execution | CSP object-src 'none' |
| Brute force login | Ratelimiter (10/min/IP) + account lockout (5/15min) |
| Mass assignment | InputFactory — nu poți pasa cîmpuri nedefinite |
| Parameter pollution | Parser propriu (parser.rs) — nu serde_urlencoded |
| Query param injection | QueryValidator + InputFactory::parse_search |

## Diagrama fluxului unui request (exemplu: adăugare în coș)

```
Browser                         Aplicație
  │                                │
  │  POST /cart/add                │
  │  product_slug=samsung-s22      │
  │  qty=1                         │
  │                                │
  │───────────────────────────────▶│
  │                                │
  │  ┌─── GRANIȚĂ (NEÎNCREDERE)   │
  │  │ body: String (RAW)         │
  │  │ parse_any_into()           │  ← parser.rs
  │  │ get_field("product_slug")  │  ← extrage
  │  │ InputFactory::parse_slug() │  ← Slug garantat valid
  │  │ InputFactory::parse_qty()  │  ← Quantity garantat valid
  │  └────────────────────────────│
  │  │ LogicFactory::verify_qty() │  ← qty în range
  │  │ LogicFactory::verify_stock │  ← stock suficient
  │  └────────────────────────────│
  │  │ ADD to DB (INSERT ON CONFLICT) │ ← LEGO module
  │  └────────────────────────────│
  │  │ OutputFactory::safe_redirect │ ← 302 sigur
  │  │ OutputFactory::safe_cookie │ ← Set-Cookie sigur
  │  └─── GRANIȚĂ (IEȘIRE) ───────│
  │                                │
  │◀──────────────────────────────│
  │  302 /products                │
  │  Set-Cookie: session_id=...   │
```

## Log-uri de securitate (ce apare în producție)

```
WARN  logic::idor: IDOR încercat: user=... owner=... object=order
WARN  query: page invalid: -1 (folosesc default 1)
WARN  query: token invalid format: abc (ignorat)
WARN  query: session_id suspect: 300 caractere (ignorat)
WARN  ratelimit: Rate limit depășit pentru 192.168.1.4
WARN  auth::ratelimit: Rate limit signup de la IP=...
WARN  cart::add: InputFactory: InvalidSlug("...")
ERROR orders::stripe: Stripe checkout eșuat: ...
```