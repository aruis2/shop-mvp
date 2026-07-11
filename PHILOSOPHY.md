# 🏛️ Shop-MVP — Filosofie și Obiective

## Filozofia

**"Simplitate, respect pentru standarde, prevenirea erorilor și o experiență curată pentru utilizator."**

### Principii

#### 1. Server-side first
Zero JavaScript în producție. Totul e server-side rendering cu form POST + 302 redirect (PRG pattern). Fiecare request produce HTML complet și final. Fără stare client-side, fără API calls după primul paint.

#### 2. Respect pentru standarde
HTTP status codes corecți (302, 401, 403, 429). Cookie HttpOnly pentru JWT. CSP, X-Frame-Options, X-Content-Type-Options, Referrer-Policy. Fără hack-uri, fără soluții "creative".

#### 3. Prevenirea erorilor
Rate limiting pe login/signup. Validare server-side la fiecare formular. Erori afișate uniform cu cutie roșie și link "← înapoi". Panic hook care scrie în fișier. Request ID pentru trasabilitate.

#### 4. Experiență curată
Fără mesaje criptice. Fără 500-uri fară sens. Fiecare acțiune → feedback vizibil. Fiecare eroare → soluție (back link). Fără loading spinners, fără waterfall de requesturi.

#### 5. HN Philosophy (Hacker News)
Text e mai rapid decât JavaScript. O pagină din 2007 e mai robustă decât un SPA din 2026. Conținutul e text, nu aplicație. Cacheabil, accesibil, indexabil.

#### 6. seL4-inspired capability architecture
Fiecare handler primește doar capabilitățile de care are nevoie (AuthState, ProductState, etc.), nu AppState întreg. Domenii izolate, verificabile la compilare.

#### 7. Hot Path optimization
Identificarea și optimizarea căii principale (hot path) prin sistem: DB query-uri, render, plăți. Orice nu e pe hot path poate fi amînat sau făcut asincron.

#### 8. LEGO modular architecture
Fiecare componentă e un crate independent (9 crate-uri), asamblate ca piese LEGO prin trait-uri. Înlocuiești o implementare fără să afectezi restul.

#### 9. Dual-project ecosystem
`shop-mvp` = aplicația propriu-zisă. `myapp` = monorepo cu toate tool-urile, articolele, router-ul. Separare clară între produs și infrastructură.

#### 10. Cross-compilation first
Build pentru 3 target-uri: x86 (desktop/Cloud Run), ARM (S22), aarch64 (Android). Toate din același cod sursă.

#### 11. Security in layers (defense in depth)
Niciun strat nu e de ajuns. Headere → middleware → capability-based → rate limiting → lockout → audit log. Dacă unul cedează, următorul prinde.

#### 12. Git history first — fiecare pas e un commit
Orice schimbare, oricît de mică, trece prin git. Commituri mici, frecvente, descriptive. Fiecare commit spune o poveste: *ce s-a schimbat și de ce*. Git e sursa unică de adevăr pentru istoricul proiectului. Fără cod pierdut, fără "cum era înainte?". Dacă nu e în git, nu există.

#### 13. Comentarii avansate cu referințe — codul documentează singur de ce
Fiecare bucată de cod poartă comentarii care explică **de ce** e scris așa, nu doar **ce** face. Comentariile fac referință la:
- **Standarde**: `// OWASP ASVS V3.3.1`, `// CIS Control 7`, `// WCAG 2.1 2.4.1`
- **Filosofie**: `// HN Philosophy: zero JS in production`, `// seL4: capability-based`
- **Rațiune tehnică**: `// i64 intermediar previne overflow`, `// PhantomData = 0 bytes`
- **Bug-uri prevenite**: `// previne comenzi duplicate (PRG pattern)`, `// previne SQL injection`

Un comentariu bun răspunde la întrebarea pe care și-o pune cineva peste 6 luni cînd citește codul. Fără comentarii de tipul `// i++` — aia se vede și din cod.

#### 14. Inputul utilizatorului e prins la granița aplicației — zero intermediari
Tot ce vine de la utilizator (HTTP body, query params, cookie-uri) e prelucrat de **codul nostru, la prima linie** în handler. Nu folosim librării externe pentru parsarea inputului brut — avem propriul parser URL-encoded (`types/parser.rs`), propriul cookie parser (`cookie.rs`), propriile tipuri cu validare (`types/email.rs`, `types/price.rs`, etc.).

Avantaje:
- **Control total**: știm EXACT cum e prelucrat inputul, fără surprize din dependințe externe
- **Zero risc de securitate din crate-uri terțe**: niciun CVE într-un parser scris de noi nu ne poate afecta
- **Fix la graniță**: între `body: String` (de la Axum) și primul nostru `InputFactory::parse_*()` nu există niciun pas intermediar — nici serde_urlencoded, nici un alt crate
- **Dacă apare o problemă, o rezolvăm în 1 minut**: nu așteptăm update de la terți

#### 15. Conștientizarea gap-ului Rust — ce NU prinde compilatorul și cum luptăm

**Problema:** Rust elimină la compilare ~70% din clasele de bug-uri (memorie, null, tipuri, concurență la date). Dar **~30% rămîn** — bug-uri pe care compilatorul NU le poate detecta pentru că nu țin de sintaxă sau tipuri, ci de **semnificația logică** a programului.

**BUG-URI PE CARE RUST NU LE PRINDE (și de ce):**

| Categorie | Exemplu | De ce nu prinde Rust | Cum luptăm noi |
|---|---|---|---|
| **🔴 IDOR** | Userul A plătește comanda userului B | Compilatorul nu știe că `order.user_id` trebuie să fie `current_user.id` | `LogicFactory::verify_ownership()` |
| **🔴 State machine** | Plătești comanda deja plătită | `payment_status` e String, orice valoare e validă sintactic | `LogicFactory::verify_not_paid()` |
| **🔴 Authorization** | User obișnuit accesează `/admin` | `role` e String, `"user"` e la fel de valid ca `"admin"` | Capability-based state + `LogicFactory::verify_admin()` |
| **🟡 Race condition** | Doi useri cumpără ultimul produs | Compilatorul vede cod secvențial, nu știe de concurență reală | `SELECT ... FOR UPDATE` + tranzacții |
| **🟡 Business logic** | Preț total depășește soldul | `Price` + `Quantity` sînt valide individual, combinația nu | `LogicFactory::verify_max_order_value()` |
| **🟡 Resource leak** | Conexiuni DB neînchise | sqlx le închide automat, dar nu și fișiere/stream-uri | Drop trait + manual review |
| **🟡 Panic** | `unwrap()` pe `None` neașteptat | Compilatorul nu știe dacă `None` e posibil la runtime | `?` operator, pattern matching, evitare unwrap |
| **🟡 Performanță** | Query N+1, algoritm O(n²) | Orice algoritm compilează, indiferent de viteză | Profiling (`perf`, flamegraph) |

**De ce nu ne-am apucat din prima de asta?**

Pentru că **InputFactory + OutputFactory** sînt fundația — fără ele, orice regulă de business e inutilă:
1. Dacă nu parsezi inputul, intri cu date invalide în LogicFactory
2. Dacă nu sanitarizezi outputul, rulele de business nu contează — XSS-ul rulează oricum
3. Abia DUPĂ ce ai granițele sigure (input/output), poți construi reguli de business fără să reinventezi roata

Ordinea corectă e:
```
InputFactory (parse) → OutputFactory (sanitizare) → LogicFactory (reguli)
   ─── faza 1 ✅ ───     ─── faza 2 ✅ ───       ─── faza 3 (acum) ───
```

**Ce am ratat în STANDARDS.md și filosofie:**

Standarde care cer explicit verificări pe care Rust nu le poate face:
- **OWASP ASVS V2.8** — Verificare că token-ul aparține userului (IDOR) — noi o facem ad-hoc, nu sistematic
- **OWASP ASVS V3.3** — Session management — nu verificăm expiry la fiecare request
- **OWASP ASVS V4.2** — State machine for orders — nu avem diagramă formală
- **OWASP API Top 10 #4** (Resource exhaustion) — rate limiting avem, dar nu și verificare de stock în tranzacție

**Soluția:** LogicFactory — o uzină separată care centralizează TOT ce Rust nu poate verifica la compilare:
- Ownership (IDOR)
- Authorization (role, admin)
- State machine (status transitions)
- Business rules (stock, price limits)
- Rate limiting  
- Idempotency

**Regulă nouă pentru cod:** Orice verificare care NU ține de sintaxa/tipul datelor (deci nu poate fi prinsă de compilator) TREBUIE să treacă printr-o metodă `LogicFactory::verify_*()`. Fără excepții. Dacă în cod scrii `if order.user_id != current_user.id` direct, ai greșit — trebuie `LogicFactory::verify_ownership()`.

---

## Articole conexe

Pentru aprofundarea fiecărui principiu:
- `articles/filosofia-hn-server-side.md` — Filozofia HN
- `articles/arhitectura-zero-js.md` — Zero JavaScript
- `articles/prg-pattern-impl.md` — PRG pattern
- `articles/arhitectura-lego-hotpath.md` — LEGO + Hot Path
- `articles/arhitectura-dual-project-myapp-shop-mvp.md` — Dual project
- `articles/hot-path.md` — Hot Path optimization
- `articles/cross-compilare.md` — Cross-compilation
- `articles/strategie-securitate-nivele.md` — Security levels
- `articles/securitate-informatica.md` — Securitate informatică
- `articles/ce-este-enterprise.md` — Ce înseamnă Enterprise
- `articles/mold-sccache-advanced.md` — mold + sccache
- `articles/pgvector-android-termux.md` — pgvector pe Android
- `articles/webassembly-prod.md` — WebAssembly
- `articles/ubuntu-dev-optimizare.md` — Ubuntu dev setup

---

## Obiective

### Fază 1 — Fundamental (completat ✅)
- [x] Arhitectură capability-based
- [x] Zero JS production
- [x] PRG pattern peste tot
- [x] JWT HttpOnly cookie auth
- [x] DetectBasePath (reverse proxy)
- [x] Login/signup cu redirect

### Fază 2 — Stabilitate (completat ✅)
- [x] Security headers (CSP, X-Frame-Options, etc.)
- [x] Health check cu DB
- [x] Graceful shutdown
- [x] Body size limit (2MB)
- [x] Rate limiting (10 req/min/IP)
- [x] Debug logging (request ID, SQL timing, panic hook)
- [x] Template error logging
- [x] Startup config + route listing
- [x] Defense in depth — 8+ layere de securitate
- [x] Cross-compilation: x86 + ARM + aarch64

### Fază 3 — Funcțional (completat ✅)
- [x] Stock indicators
- [x] Category browsing
- [x] Paginare (admin + user orders)
- [x] Erori stylizate uniform
- [x] Standalone copy
- [x] LEGO modular (9 crate-uri)
- [x] Hot path optimization
- [x] PGVector semantic search
- [x] Dual-project: shop-mvp + myapp
- [x] mold + sccache build pipeline

### Fază 4 — Enterprise (în curs)
- [ ] Sitemap.xml + robots.txt
- [ ] JSON-LD produse
- [ ] Factură/Recepisă
- [ ] Anulare comandă de către user
- [ ] Backup DB automat (✅ backup-db.sh)
- [ ] Notificare email
- [ ] Wishlist
- [ ] Filtre căutare avansate
- [ ] CSRF protection (✅ implementat)
- [ ] Watchdog/auto-restart
- [ ] WebAssembly production
- [ ] Unificare myapp + shop-mvp

---

**Motto:** *"Mai puțin înseamnă mai mult. Zero JS e mai stabil decât orice librărie."*
