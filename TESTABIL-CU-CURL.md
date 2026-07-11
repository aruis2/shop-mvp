# Testabil cu `curl` — Filosofia zero-JS

> **Un sistem testabil cu `curl` e un sistem bun.**
> Orice altceva e o datorie tehnică amânată.

---

## Ce înseamnă "testabil cu `curl`"

Un sistem web este testabil cu `curl` dacă:

1. **Fiecare pagină** = un singur `GET` → returnează HTML complet
2. **Fiecare acțiune** = un `POST` → face efectul + `302 Redirect`
3. **Fiecare răspuns** poate fi inspectat cu `curl -v`, fără browser

```bash
# Navigare pagină
curl -v http://localhost:3001/products

# Login
curl -v -X POST -d "email=test@test.com&password=abc" http://localhost:3001/login

# Adăugare în coș
curl -v -X POST -b cookies.txt -d "slug=produs-1&qty=1" http://localhost:3001/cart/add

# Checkout
curl -v -X POST -b cookies.txt http://localhost:3001/orders/checkout
```

---

## La ce duce când NU poți testa cu `curl`

### 1. Bug-uri invizibile până în producție

Dacă ai nevoie de browser ca să testezi, înseamnă că ai **state client-side** care poate fi inconsistent cu serverul. Bug-urile apar doar în anumite secvențe de click-uri, greu de reprodus.

**Exemplu real:** HTMX `hx-headers` gol trimitea `X-Session-Id: ""` — serverul interpreta ca și cum n-ar fi session, crea un coș nou la fiecare request. Cu `curl`, vezi imediat header-ele trimise.

### 2. Testare lentă și fragilă

Testele end-to-end cu Playwright/Cypress sunt:
- Lente (deschid browser, așteaptă selecție, screenshot)
- Fragile (un selector CSS se schimbă → testul crapă)
- Greu de rulat în CI (au nevoie de un browser headless + display)

Cu `curl`, un test e o linie de shell:
```bash
assert_eq($(curl -s -o /dev/null -w "%{http_code}" $URL), 200)
```

### 3. Debugging complicat

Un bug raportat de utilizator:
- **Cu JS/HTMX**: "Am dat click pe buton și nu s-a întâmplat nimic" → Trebuie să reproduci exact aceeași sesiune, să inspectezi network tab, să verifici dacă HTMX a făcut request-ul corect, dacă serverul a răspuns cum trebuie, dacă HX-Redirect a fost procesat corect...
- **Cu PRG + curl**: "Am dat click și m-a dus la pagina greșită" → `curl -v -X POST -b cookies.txt` și vezi exact ce răspunde serverul.

### 4. Dependency hell pe client

Framework-urile JS client vin și pleacă:
- jQuery → Angular → React → Vue → Svelte → HTMX →...
- Fiecare aduce bug-uri specifice, breaking changes, migrări dureroase

**Standarde stabile** (HTML, HTTP, URL) sunt aceleași de 30 de ani. Un formular `<form method="POST" action="...">` funcționează identic în orice browser, acum și peste 10 ani.

### 5. Bloating inevitabil

Odată ce ai un framework client, ajungi să:
- Adaugi un script pentru auth
- Altul pentru coș
- Altul pentru notificări
- Altul pentru analytics
- ...

Fiecare script adaugă complexitate, timp de încărcare, vulnerabilități și moduri de eșec.

---

## Standarde stabile pe care ne bazăm

| Standard | RFC / Spec | Vârstă |
|----------|-----------|--------|
| HTTP/1.1 | RFC 7230-7235 | ~25 ani |
| HTTP/2 | RFC 7540 | ~10 ani |
| URL | RFC 3986 | ~20 ani* |
| HTML5 | WHATWG Living Standard | ~15 ani |
| Cookie | RFC 6265 | ~15 ani |
| POST/Redirect/GET (PRG) | Pattern web | ~20 ani |

*RFC 3986 înlocuiește RFC 2396 (1998) și RFC 1738 (1994)

Aceste standarde NU se schimbă de la o lună la alta. Un `<form>` cu `method="POST"` va funcționa la fel în 2030 ca și azi.

---

## Teorema: `curl` = Browser (1:1)

Cu PRG pur, **nu există diferență** între un request făcut cu `curl` și unul făcut de browser:

```
curl  → GET  /products          → HTML complet
Browser → GET /products          → același HTML (identic)

curl  → POST /cart/add + cookie → 302 /cart
Browser → POST (form submit)     → 302 /cart (redirect automat)
```

De ce sunt identice:
- **Zero JS** care să modifice DOM-ul după load
- **Zero HTMX** care să facă request-uri diferite decât browserul
- **Zero state client-side** care să difere de server
- Un `<form method="POST" action="/cart/add">` face EXACT același request ca `curl -X POST -d "slug=x&qty=1"`

**Corolar:** Dacă merge în `curl`, merge garantat în orice browser. Orice diferență e un bug de arhitectură.

## Avantaje concrete

| Caracteristică | PRG + curl | JS framework |
|----------------|-----------|-------------|
| Testare | `curl \| grep` | Playwright/Cypress |
| Debug | `curl -v` | Network tab + console |
| Bug-uri ascunse | Puține (totul e explicit) | Multe (state client-server) |
| Dependente externe | Zero | Zeci de npm packages |
| Timp de încărcare | < 100ms | >= 500ms (JS bundle) |
| Funcționează fără JS | Da | Nu |
| Funcționează peste 10 ani | 100% garantat | Cine știe? |
| **curl = browser?** | ✅ Da, 1:1 | ❌ Niciodată |

---

## Concluzie

> **Mai puțin înseamnă mai mult.**

Un sistem testabil cu `curl` e mai simplu, mai rapid, mai ieftin de întreținut și mai robust. Nu e o limitare — e o **alegere conștientă** de a construi pe standarde stabile, nu pe trenduri trecătoare.

Dacă nu poți testa cu `curl`, ai o problemă de arhitectură.
