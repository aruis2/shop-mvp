# Arhitectura Aplicațiilor Web Sigure și Robuste

## Un Tratat de Inginerie Software Aplicată

---

### Despre această lucrare

Prezentul tratat reprezintă o sinteză a principiilor, pattern-urilor și standardelor care stau la baza construcției aplicațiilor web sigure și robuste. Materialul este structurat pe niveluri de abstractizare — de la fundamentele teoretice ale securității și corectitudinii, până la implementări practice și standarde industriale.

**Publicul țintă:** Ingineri software, arhitecți de sisteme, specialiști în securitate informatică, studenți ai facultăților de profil.

**Prerechizite:** Cunoștințe solide de programare (de preferință Rust), înțelegerea de bază a protocolului HTTP și a arhitecturii client-server.

---

### Structura lucrării

```
Volumul I:    Fundamente Teoretice
            ├── Capitolul 1:  Costul unui bug și ingineria calității
            ├── Capitolul 2:  PRG Pattern și idempotența în HTTP
            ├── Capitolul 3:  Server-Side First Architecture
            ├── Capitolul 4:  Capability-Based Architecture
            └── Capitolul 5:  Port-Adapter (Hexagonal) Architecture

Volumul II:   Tipuri și Corectitudine
            ├── Capitolul 6:  Parse, Don't Validate
            ├── Capitolul 7:  Type-State Pattern
            ├── Capitolul 8:  Newtype Pattern
            └── Capitolul 9:  Phantom Types

Volumul III:  Verificare și Validare
            ├── Capitolul 10: Property-Based Testing
            ├── Capitolul 11: Fuzz Testing
            ├── Capitolul 12: Snapshot Testing
            └── Capitolul 13: Testarea Integrării

Volumul IV:   Securitate Web
            ├── Capitolul 14: OWASP ASVS Level 1
            ├── Capitolul 15: HSTS
            ├── Capitolul 16: Content Security Policy
            ├── Capitolul 17: CORS
            ├── Capitolul 18: CSRF
            ├── Capitolul 19: Rate Limiting
            └── Capitolul 20: STRIDE Threat Model

Volumul V:    Arhitectură Avansată
            ├── Capitolul 21: Defense in Depth
            ├── Capitolul 22: Zero Trust Architecture
            ├── Capitolul 23: Observability
            ├── Capitolul 24: Error Handling
            └── Capitolul 25: Non-Repudiation

Volumul VI:   Anexe și Referințe
            ├── Anexa A: Checkilist ASVS Level 1
            ├── Anexa B: Matricea STRIDE Completă
            ├── Anexa C: Teoreme Arhitecturale
            ├── Glosar de Termeni
            └── Referințe Bibliografice
```

---

# Volumul I: Fundamente Teoretice

---

## Capitolul 1: Costul unui Bug și Ingineria Calității

### 1.1 Principiul fundamental

În ingineria software, costul corectării unui defect crește exponențial cu timpul scurs de la momentul introducerii sale până la momentul detectării. Acest fenomen, documentat pentru prima dată de Barry Boehm în 1976 și confirmat ulterior de studii IBM, Microsoft și NIST, poartă numele de **Legea lui Boehm**.

Formal, costul $C$ al corectării unui bug în funcție de faza $f$ în care este detectat poate fi exprimat ca:

$$C_f = C_0 \cdot \kappa^{f}$$

unde:
- $C_0$ este costul corectării în faza de compilare
- $\kappa \approx 10$ este factorul de multiplicare pe fază
- $f$ este numărul de faze prin care bug-ul a trecut

Această lege are o implicație profundă: **strategia optimă de gestionare a calității nu este detectarea cât mai multor bug-uri, ci prevenirea existenței lor**.

### 1.2 Clasificarea defectelor

Din perspectiva severității, defectele pot fi clasificate în cinci categorii:

1. **Blocante** — Împiedică complet funcționarea sistemului. Cost: pierderea totală a venitului pe durata avariei.
2. **Critice** — Compromit date sau securitate. Cost: $10-100 per înregistrare expusă.
3. **Importante** — Afectează experiența utilizatorului. Cost: indirect, de 10× valoarea tranzacției.
4. **Medii** — Inconveniente minore. Cost: pierdere de timp a utilizatorului.
5. **Ușoare** — Defecte cosmetice. Cost: minim, dar cumulat poate afecta încrederea.

### 1.3 Metrica DRE

**Defect Removal Efficiency (DRE)** măsoară eficiența procesului de detectare:

$$DRE = \frac{D_{\text{pre-producție}}}{D_{\text{pre-producție}} + D_{\text{producție}}} \times 100$$

O DRE de 95% înseamnă că 95 din 100 de bug-uri sunt găsite înainte de a ajunge la utilizatori. Pentru un sistem de producție, ținta minimă este 95%.

### 1.4 Studii de caz relevante

**Knight Capital (2012).** Un flag boolean reutilizat a generat pierderi de 440 milioane USD în 45 de minute. Cauza: un operator boolean a fost folosit pentru două scopuri diferite, iar codul vechi de 9 ani a fost activat din greșeală.

**Ariane 5 (1996).** Un integer overflow la conversia unui număr pe 64 de biți la 16 biți a dus la explozia rachetei la 40 de secunde după lansare, generând o pierdere de 370 milioane USD.

**Therac-25 (1985-1987).** Un race condition între două procese concurente a dus la administrarea letală de radiații pentru trei pacienți. Cauza: o variabilă booleană nesincronizată.

**Heartbleed (2014).** O eroare de verificare a lungimii buffer-ului în OpenSSL a permis atacatorilor să citească memoria serverelor, afectând aproximativ 17% din internet. Cost estimat: 500 milioane USD.

### 1.5 Concluzii

**Principiul shift-left:** Detectarea și prevenirea bug-urilor trebuie să aibă loc cât mai devreme posibil în ciclul de dezvoltare. Ideal, la nivel de compilare, prin intermediul sistemului de tipuri și al verificărilor statice.

---

## Capitolul 2: PRG Pattern și Idempotența în HTTP

### 2.1 Fundament teoretic

Protocolul HTTP definește în mod explicit idempotența metodelor sale. Conform RFC 7231:

- **GET, HEAD, PUT, DELETE, OPTIONS** sunt idempotente — execuția repetată produce același efect
- **POST, PATCH** nu sunt idempotente — execuția repetată poate produce efecte diferite

**Problema practică:** Un formular web trimis prin POST poate fi re-transmis accidental prin:
1. Reîmprospătarea paginii (F5/Ctrl+R)
2. Navigarea înapoi și re-trimiterea
3. Dublul click pe butonul de submit
4. Reîncercarea automată a browserului la timeout de rețea

### 2.2 Pattern-ul Post-Redirect-Get

Soluția constă în separarea procesării datelor (POST) de afișarea rezultatului (GET):

```
                        Cerere inițială
                              │
                              ▼
                    ┌──────────────────┐
                    │  POST /checkout  │
                    │  (procesează)    │
                    └────────┬─────────┘
                             │
                     302 Found ──────┐
                  Location: /success │
                             │       │
                             ▼       │
                    ┌──────────────┐  │
                    │  GET /success │◄┘
                    │  (afișează)   │
                    └──────────────┘
```

**Proprietăți garantate:**
1. POST-ul se execută o singură dată
2. Reîmprospătarea (F5) re-execută doar GET-ul
3. Navigarea înapoi nu re-trimite POST-ul
4. Istoricul browserului conține doar GET-uri

### 2.3 Reguli de implementare

1. Orice handler POST trebuie să răspundă cu `302 Found`
2. Orice răspuns 302 trebuie să includă header-ul `Location`
3. Orice eroare în procesare trebuie să redirecționeze cu parametrul `?error=`
4. Orice URL de destinație trebuie să fie un GET care funcționează

---

## Capitolul 3: Server-Side First Architecture

### 3.1 Principiul minimalismului JavaScript

Un sistem server-side pure minimizează suprafața de atac și elimină clase întregi de bug-uri specifice execuției client-side:

**Bug-uri eliminate prin absența JavaScript-ului:**
- Stare client-server desincronizată
- Race condition-uri între execuția JS și procesarea cookie-urilor
- Diferențe de comportament între browsere
- Vulnerabilități XSS bazate pe execuția de cod inline
- Pierderea stării la reîmprospătare

### 3.2 Avantaje cantitative

| Dimensiune | Client-side | Server-side | Factor |
|-----------|-------------|-------------|--------|
| Timp până la interactivitate | 1-3s | ~50ms | 20-60× |
| Bug-uri de stare per sprint | 3-5 | 0 | ∞ |
| Consum baterie (mobil) | 2-5% per pagină | 0.1% per pagină | 20-50× |
| Complexitate arhitecturală | 3 layer-e | 1 layer | 3× |

---

## Capitolul 4: Capability-Based Architecture

### 4.1 Fundamente teoretice

Arhitectura bazată pe capabilități își are originea în sistemul de operare seL4 — un microkernel verificat formal care demonstrează matematic că un proces nu poate accesa resurse pentru care nu deține o capabilitate explicită.

**Definiție:** O capabilitate este un token intangibil, verificat de compilator, care conferă posesorului său dreptul de a accesa o resursă specifică.

### 4.2 Corespondența cu arhitectura web

| Concept seL4 | Echivalent web |
|-------------|---------------|
| Proces | Handler HTTP |
| CNode (Capability Node) | Domain State |
| Kernel | Router (cu `with_state`) |
| Verificare formală | Compilator |

### 4.3 Principiul de funcționare

Fiecare handler primește exclusiv capabilitățile de care are nevoie, sub forma unui domain state specific. Un handler de autentificare nu poate accesa plăți nu pentru că „nu ar trebui", ci pentru că **nu poate** — tipul său de state nu conține un referință către subsistemul de plăți.

```rust
// Un handler de login poate accesa doar AuthState
async fn login_handler(State(state): State<AuthState>) -> Response {
    state.auth.verify(...)?;   // ✅ Permis
    state.payment.refund(...)?; // ❌ ERROR la compilare
}
```

---

## Capitolul 5: Port-Adapter (Hexagonal) Architecture

### 5.1 Principiul

Arhitectura hexagonală, descrisă de Alistair Cockburn în 2005, stabilește că o aplicație comunică cu exteriorul exclusiv prin **port-uri** (interfețe), iar **adaptoarele** implementează aceste port-uri pentru tehnologii concrete.

### 5.2 Avantaje structurale

1. **Independentă de tehnologie** — Schimbarea bazei de date sau a provider-ului de plată nu afectează logica de business
2. **Testabilitate** — Orice port poate fi implementat de un adaptor de test (mock)
3. **Izolare** — O vulnerabilitate într-un adaptor nu se propagă în restul sistemului

---

# Volumul II: Tipuri și Corectitudine

---

## Capitolul 6: Parse, Don't Validate

### 6.1 Principiul fundamental

Există o diferență fundamentală între **validare** și **parsare**:

- **Validarea** verifică datele și le păstrează în tipul lor original. De exemplu, verifici că un string conține caracterul `@`, dar îl păstrezi ca `String`. Orice funcție ulterioară poate primi accidental un string neverificat.
- **Parsarea** transformă datele într-un tip nou care **garantează** prin însăși existența sa că datele sunt valide. Un `Email` parsat nu mai poate fi invalid — tipul însuși este dovada validității.

### 6.2 Implementarea

Pentru fiecare tip de date primite de la utilizator, se definește un tip nou cu o metodă `parse()`:

```rust
pub struct Email(String);

impl Email {
    pub fn parse(s: &str) -> Result<Self, Error> {
        // Validare + transformare → tip nou garantat valid
    }
}
```

După parsare, niciun cod ulterior nu mai trebuie să verifice validitatea — tipul o garantează.

---

## Capitolul 7: Type-State Pattern

### 7.1 Concept

Type-state pattern codifică stările unui sistem în sistemul de tipuri al limbajului de programare. Tranzițiile invalide nu sunt sancționate la runtime, ci sunt **imposibile** la compilare.

### 7.2 Exemplu: Ciclul de viață al unei comenzi

```
Order<Pending> ──pay()──▶ Order<Paid> ──ship()──▶ Order<Shipped>
      │                       │
      │ cancel()              │ refund()
      ▼                       ▼
Order<Cancelled>       Order<Refunded>
```

Fiecare stare este un tip ZST (zero-sized type). Tranzițiile sunt metode care consumă starea curentă și produc starea următoare. O stare terminală nu mai are metode de tranziție.

---

## Capitolul 8: Newtype Pattern

### 8.1 Principiul

Un **newtype** este un tip nou creat prin împachetarea unui tip existent. Deosebirea față de un alias de tip este că newtype-ul creează un tip **distinct** — nu poate fi convertit implicit în tipul împachetat.

### 8.2 Aplicații

```rust
pub struct Email(String);      // Un Email nu e un String
pub struct Price(i32);         // Un Price nu e o cantitate
pub struct SessionId(Uuid);    // Un SessionId nu e un UserId
```

Fără newtype-uri, confuziile între identificatori (SessionId vs UserId vs OrderId) sunt o sursă frecventă de bug-uri.

---

## Capitolul 9: Phantom Types

### 9.1 Definiție

`PhantomData<T>` este un tip care ocupă 0 bytes în memorie, dar poartă un parametru de tip. Este utilizat atunci când un parametru generic este necesar pentru siguranța tipologică, dar nu este folosit direct de structură.

### 9.2 Aplicație: Marcarea validării

```rust
pub struct CheckoutForm<State = Unvalidated> {
    // câmpuri...
    _state: PhantomData<State>,
}

impl CheckoutForm<Unvalidated> {
    pub fn validate(self) -> Result<CheckoutForm<Validated>, Error> { }
}

impl CheckoutForm<Validated> {
    pub fn process(self) -> Result<Order, Error> { }
}
```

Un formular nevalidat nu poate fi procesat — nu pentru că o excepție ar fi aruncată la runtime, ci pentru că metoda `process()` nici măcar nu există pe `CheckoutForm<Unvalidated>`.

---

# Volumul III: Verificare și Validare

---

## Capitolul 10: Property-Based Testing

### 10.1 Limitările testării tradiționale

Testarea tradițională (example-based) verifică doar cazurile pe care programatorul le-a anticipat. Un set de 5 exemple acoperă o fracțiune infimă din spațiul posibil al intrărilor.

### 10.2 Abordarea property-based

În loc să scrie exemple, programatorul scrie **proprietăți** — afirmații care trebuie să fie adevărate pentru orice intrare validă. Un generator automat produce sute sau mii de cazuri de test:

```rust
proptest! {
    #[test]
    fn total_este_suma(qty in 1..10000u32, price in 1..999999i32) {
        let total = calculate_total(qty, price).unwrap();
        assert_eq!(total, (qty as i64) * (price as i64));
    }
}
```

---

## Capitolul 11: Fuzz Testing

### 11.1 Definire

Fuzz testing-ul constă în generarea automată de input-uri aleatoare (sau semialeatoare) și alimentarea sistemului cu acestea, cu scopul de a detecta căderi, comportamente nedefinite sau vulnerabilități de securitate.

### 11.2 Proptest vs Fuzz

| Caracteristică | Property-Based | Fuzz |
|---------------|---------------|------|
| Input | Distribuții controlate | Bytes aleatori |
| Scop | Corectitudine logică | Securitate/stabilitate |
| Viteză | ~1.000 teste/sec | ~10.000 input-uri/sec |
| Moment | La fiecare commit | Periodic (CI nocturn) |

---

## Capito 12-13: Snapshot și Testarea Integrării

Testarea prin **snapshot** salvează output-ul unui test și îl compară cu execuțiile ulterioare. Orice diferență este semnalată ca potențial bug.

Testarea **integrării** rulează contra unei baze de date reale (de preferință în Docker) pentru a verifica că interacțiunile SQL sunt corecte — lucru pe care mock-urile nu îl pot garanta.

---

# Volumul IV: Securitate Web

---

## Capitolul 14: OWASP ASVS Level 1

OWASP Application Security Verification Standard definește un set de cerințe verificabile pentru securitatea aplicațiilor web. Nivelul 1 (security baseline) cuprinde 14 capitole, de la autentificare și gestionarea sesiunilor, până la criptografie și configurare.

**Cerințe implementate în shop-mvp:**
- V2: Autentificare cu JWT HttpOnly și Argon2 hashing
- V3: Cookie-uri cu flag-urile HttpOnly, Secure, SameSite
- V4: Control de acces bazat pe capabilități
- V8.3: Cache-Control pentru date sensibile
- V9: HSTS și Referrer-Policy
- V10: CSP explicit

---

## Capitolul 15: HSTS (HTTP Strict-Transport-Security)

HSTS forțează browserul să utilizeze exclusiv HTTPS pentru conexiunile viitoare, prevenind atacurile de tip SSL stripping. Header-ul se stabilește odată la prima conexiune HTTPS și rămâne valabil pentru durata specificată (de obicei 1 an).

```
Strict-Transport-Security: max-age=31536000; includeSubDomains
```

---

## Capitolul 16: Content Security Policy (CSP)

CSP este un mecanism care specifică browserului ce resurse sunt permise și ce acțiuni pot fi executate. Fiecare directivă controlează o categorie de resurse:

- `script-src` — sursele permise pentru script-uri
- `style-src` — sursele permise pentru stiluri
- `form-action` — destinațiile permise pentru formulare
- `img-src` — sursele permise pentru imagini

---

## Capitolul 17: CORS (Cross-Origin Resource Sharing)

CORS controlează ce domenii terțe pot accesa resursele aplicației. În producție, CORS trebuie restrictionat la domeniul propriu:

```rust
CorsLayer::new()
    .allow_origin(AllowOrigin::exact("https://domeniu-propriu.ro"))
```

---

## Capitolul 18: CSRF (Cross-Site Request Forgery)

Atacurile CSRF exploatează faptul că browserul trimite automat cookie-urile asociate unui domeniu. Protecția se realizează prin:
1. Cookie-uri cu `SameSite=Lax`
2. Restricția `form-action 'self'` în CSP
3. Token-uri CSRF (opțional, când celelalte măsuri nu sunt suficiente)

---

## Capitolul 19: Rate Limiting

Rate limiting-ul limitează numărul de cereri permise într-o fereastră de timp, prevenind atacurile de brute-force și DoS:

```rust
let login_limiter = RateLimiter::new(10, 60);  // 10 req/min/IP
```

---

## Capitolul 20: STRIDE Threat Model

STRIDE este o metodologie Microsoft pentru clasificarea amenințărilor în șase categorii:

| Acronim | Categoria | Contramăsură |
|---------|-----------|-------------|
| **S**poofing | Falsificarea identității | JWT + rate limiting |
| **T**ampering | Modificarea datelor | HTTPS + body size limit |
| **R**epudiation | Negarea acțiunii | Audit log |
| **I**nformation Disclosure | Expunerea datelor | Capability-based access |
| **D**enial of Service | Refuzul serviciului | Rate limiting |
| **E**levation of Privilege | Escaladarea privilegiilor | Separarea rolurilor |

---

# Volumul V: Arhitectură Avansată

---

## Capitolul 21: Defense in Depth

Niciun strat individual de securitate nu este perfect. Defense in depth pledează pentru implementarea a multiple straturi independente, astfel încât spargerea unuia să nu compromită întregul sistem.

**Cele opt straturi recomandate:**

1. Headere de securitate (CSP, HSTS)
2. Restricții CORS și CSRF
3. Rate limiting
4. Autentificare
5. Autorizare bazată pe capabilități
6. Validare și parsare
7. PRG pattern
8. Sistemul de tipuri al limbajului

---

## Capitolul 22: Zero Trust Architecture

**Principiul fundamental:** „Never trust, always verify." Orice cerere, indiferent de originea sa, trebuie să fie autentificată, autorizată și validată.

---

## Capitolul 23: Observability

Observabilitatea se bazează pe trei piloni:
1. **Logging** — Evenimente discrete (request-uri, erori, panic)
2. **Metrics** — Agregări numerice (număr de request-uri, durată medie)
3. **Tracing** — Urmărirea fluxului unui request prin toate componentele

---

## Capitolul 24: Error Handling

Un sistem robust tratează erorile explicit, fără a le lăsa să se propage necontrolat. Categoriile de erori și răspunsurile asociate:

| Categorie | Răspuns | Exemplu |
|-----------|---------|---------|
| Validare | 302 + `?error=` | Email invalid |
| Autentificare | 401 | Token expirat |
| Autorizare | 403 | Rol insuficient |
| Resursă lipsă | 404 | Produs inexistent |
| Conflict | 409 | Email duplicat |
| Limită | 429 | Prea multe încercări |
| Internă | 500 + log | Stripe timeout |

---

## Capitolul 25: Non-Repudiation

Non-repudiația asigură că un utilizator nu poate nega o acțiune pe care a efectuat-o. Se implementează printr-un log de audit append-only:

```sql
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id UUID,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    details JSONB
);
```

---

# Volumul VI: Anexe

---

## Anexa A: Teoreme Arhitecturale

1. **Server-Side First.** Reducerea dependențelor JavaScript minimizează suprafața de atac și elimină bug-urile de stare.
2. **PRG.** Orice POST trebuie să răspundă cu 302. Un POST cu 200 HTML direct este incorect.
3. **Parse, Don't Validate.** Parsarea într-un tip nou garantat este superioară validării în tipul generic.
4. **Type-State.** Codificarea stărilor în tipuri elimină posibilitatea reprezentării stărilor invalide.
5. **Newtype.** Împachetarea tipurilor în noi tipuri previne confuziile accidentale.
6. **Capability-Based.** Un handler HTTP nu trebuie să aibă acces la resurse pe care nu le folosește.
7. **Port-Adapter.** Dependențele externe trebuie abstractizate prin trait-uri.
8. **Property-Based.** Testarea proprietăților descoperă mai multe cazuri limită decât testarea exemplelor.
9. **Fuzzing.** Funcțiile care parsează input trebuie fuzz-uite.
10. **ASVS.** ASVS Level 1 oferă un baseline de securitate verificabil.
11. **STRIDE.** Orice feature nou trebuie precedat de o analiză STRIDE.
12. **Defense in Depth.** Minimum șapte straturi de securitate independente.

---

## Anexa B: Glosar de Termeni

| Termen | Definiție |
|--------|-----------|
| **Capabilitate** | Token intangibil care conferă dreptul de a accesa o resursă |
| **DRE** | Defect Removal Efficiency — procentajul defectelor detectate înainte de producție |
| **Idempotență** | Proprietatea de a produce același efect la execuții multiple |
| **Newtype** | Tip nou prin împachetarea unuia existent, oferind siguranță tipologică |
| **Phantom Type** | Parametru de tip neutilizat direct, purtător de informație la nivel de tip |
| **PRG** | Post-Redirect-Get — pattern pentru prevenirea procesării duplicate |
| **Shift-left** | Mutarea detectării defectelor în fazele incipiente ale dezvoltării |
| **Zero Trust** | Principiu conform căruia niciun request nu este de încredere implicit |

---

## Anexa C: Referințe

1. Boehm, B. W. (1976). *Software Engineering Economics*. IEEE.
2. Klein, G. et al. (2009). *seL4: Formal Verification of an OS Kernel*. ACM SOSP.
3. King, A. (2019). *Parse, Don't Validate*.
4. Minsky, Y. (2014). *Make Illegal States Unrepresentable*. Jane Street.
5. Cockburn, A. (2005). *Hexagonal Architecture*.
6. OWASP. (2024). *Application Security Verification Standard v5.0*.
7. Microsoft. (2023). *STRIDE Threat Model*.
8. Claessen, K., Hughes, J. (2000). *QuickCheck*. ICFP.

---

> *„Mai puține bug-uri nu vin din mai multă testare.*
> *Vin din tipuri care fac bug-urile imposibile.*
> *Vin din arhitectură care izolează erorile.*
> *Vin din standarde care definesc clar ce înseamnă 'corect'."*
