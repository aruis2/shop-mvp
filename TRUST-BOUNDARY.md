# 🔐 Trust Boundary — Granița de încredere a aplicației

> Ce intră, ce iese, și în ce avem încredere.

---

## Harta graniței

```
                   ┌──────────────────────────────────────────────────────┐
                   │                NELIPSIT DE ÎNCREDERE                  │
                   │          (Outside World — necontrolat de noi)         │
                   │                                                      │
                   │  HTTP request · Browser · curl · Atacatori · Bots    │
                   │  Rețea externă · CDN · Cloud · Telefon (S22)         │
                   └──────────────────┬───────────────────────────────────┘
                                      │
                          body: String (Axum)
                          query params: String
                          cookie header: String
                          ─── totul e RAW, neverificat ───
                                      │
                    ╔═════════════════╧══════════════════╗
                    ║          GRANIȚA APLICAȚIEI         ║
                    ║  (Trust Boundary — aici DECIDEM noi) ║
                    ╚═════════════════╤══════════════════╝
                                      │
              ┌───────────────────────┼───────────────────────┐
              │                       │                       │
         parser.rs              cookie.rs              InputFactory
         parse_form()           get_cookie()           parse_email()
         get_field()            set_cookie()           parse_price()
         parse_form_into()      remove_cookie()        parse_qty()
                                                        parse_phone()
                                                        parse_slug()
                                                        etc.
              │                       │                       │
              └───────────────────────┼───────────────────────┘
                                      │
                    ╔═════════════════╧══════════════════╗
                    ║        ZONA DE ÎNCREDERE           ║
                    ║   (Trusted — toate datele sînt     ║
                    ║    garantat valide de tipuri)      ║
                    ╚═════════════════╤══════════════════╝
                                      │
                   ┌──────────────────┼──────────────────┐
                   │                  │                  │
               Handlere             DB              Templates
               (Email, Price,   (deja validate    (Tera, cu
                Quantity...)    la intrare)        auto-escape)
                   │                  │                  │
                   └──────────────────┼──────────────────┘
                                      │
                   ╔══════════════════╧══════════════════╗
                   ║      IEȘIRE (spre exterior)         ║
                   ║      ← 302 redirect                 ║
                   ║      ← 200 HTML (Tera auto-escape)  ║
                   ║      ← 401/403/429 Status codes     ║
                   ║      ← Set-Cookie (HttpOnly)        ║
                   ╚═════════════════════════════════════╝
```

---

## Ce e în afara graniței (NEÎNCREDERE)

| Componentă | De ce nu avem încredere |
|-----------|------------------------|
| **Browser** | Orice browser poate trimite orice — cookie-uri modificate, form-uri false, header-e false |
| **Rețea** | HTTP poate fi interceptat — de asta avem HTTPS + HSTS |
| **curl / API calls** | Oricine poate face request-uri — nu știm cine e |
| **S22 (telefon)** | E o mașină separată, rețea locală — dar tot nu controlăm ce rulează acolo |
| **CDN / Cloud** | Nu controlăm serverele intermediare |
| **DB (remote)** | Doar cînd e în aceeași mașină avem încredere — remote DB e prin SSH tunel (WireGuard) |

## Ce e la graniță (PARSAREA)

| Componentă | Ce face | De ce e la graniță |
|-----------|---------|-------------------|
| **`parser.rs`** | Parsează URL-encoded body în `FormField[]` | E primul contact cu inputul — zero dependințe externe |
| **`cookie.rs`** | Citește cookie-uri din header | Header-ele vin direct de la browser, neverificate |
| **`InputFactory`** | Validatează fiecare cîmp în tipul său | Transformă `String` în `Email`, `Price`, etc. — odată transformat, e garantat valid |

## Ce e în zona de încredere (TRUSTED)

| Componentă | De ce avem încredere |
|-----------|---------------------|
| **Handlere** | Primesc doar tipuri sigure (`Email`, `Price`, etc.) — garantat valide |
| **DB (local)** | Noi am scris datele — le citim în aceleași tipuri |
| **Templates (Tera)** | Auto-escape HTML — XSS imposibil |
| **`state.rs`** | Capability-based — handler-ele nu pot accesa ce nu trebuie |
| **Logger** | Scrie în fișier local, append-only |
| **Backup script** | Script intern, rulează local |

## Regula de aur

> **Tot ce trece granița dinspre exterior spre interior trece prin parser.rs + InputFactory.**
> **Tot ce iese din interior spre exterior e controlat (302 redirect, Tera auto-escape, HttpOnly cookie).**
> **DB e sursă de încredere doar pentru citire — scrierea s-a făcut deja prin InputFactory.**
> **Dacă nu e verificat la graniță, nu există în interior.**

## Diagrama fluxului unui request

```
Browser                         Aplicație
  │                                │
  │  POST /login                   │
  │  email=ion%40test.com          │
  │  password=abc123               │
  │───────────────────────────────▶│
  │                                │
  │           ┌── GRANIȚĂ ───┐    │
  │           │ body: String  │    │  ← primul contact, RAW
  │           │ parser.rs    │    │  ← parsează form-ul (nostru)
  │           │ get_field()  │    │  ← extrage "email", "password"
  │           │ InputFactory │    │  ← Email::parse(), validare
  │           └── TRUSTED ───┘    │
  │           │ Email, String     │  ← garantat valide
  │           │ handler → DB     │
  │           │                   │
  │◀──────────────────────────────│
  │  302 → / (cookie HttpOnly)    │
```

