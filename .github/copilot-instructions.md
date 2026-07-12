# Flow Build → Test

Acest proiect Rust (shop-mvp) are un flow specific de build și testare.

## Build

- **`cargo check -p shop-mvp`** — verificare rapidă (1-2 secunde). Folosește asta pentru verificare rapidă în timpul dezvoltării. NU produce binary.
- **`cargo build -p shop-mvp`** — compilare completă (~20-25 secunde). Folosește doar când trebuie să rulezi efectiv.
- **`cargo build --release`** — doar pentru deploy, niciodată în dev.

## Testare

### Teste unitare
```bash
cargo test -p shop-mvp          # doar shop-mvp
cargo test --workspace          # tot workspace-ul
cargo test -p shop-mvp -- <nume_test>  # filtru
```

### Teste comportamentale (STANDARD — singurul test important)
- **`test-behavior.sh`** — 11 scenarii utilizator, ~113+ teste. Rulează pe localhost:3001.

**Asta e singura suită de teste care contează.**
Rulează `test-behavior.sh` după ORICE modificare. Dacă trece, e gata.

### test-curl.sh — NU se rulează zilnic (doar pentru securitate)
Nu-l mai folosi în flow-ul zilnic. test-behavior.sh e standardul.

**Rolul lui**: `test-curl.sh` testează din perspectiva unui atacator — endpoint-uri individuale,
headere, status codes, CSRF, rate limit, lockout. E util mai târziu pentru **teste de securitate**
când vom face pentesting explicit.

**De ce nu-l rulăm zilnic**: test-curl.sh verifică disponibilitatea serverului ("răspunde cu 200?").
Asta e exact ce face un atacator: "serverul răspunde? bun, acum caut o gaură".
test-behavior.sh verifică comportamentul aplicației din perspectiva utilizatorului
("vede utilizatorul mesajul de eroare? butonul funcționează?").

| Scop | Test | Perspectivă |
|------|------|-------------|
| Zilnic | test-behavior.sh | Utilizatorul |
| Securitate (mai târziu) | test-curl.sh | Atacatorul |

### Teste Playwright
- În `shop-mvp/tests/` — nu ruleza.

## Dev workflow complet
```bash
./dev-test.sh   # build + restart server + rulează test-behavior.sh
```

## Reguli importante
1. `cargo check` ≠ `cargo build` — check nu produce binary. Dacă vrei să rulezi, trebuie build.
2. Serverul trebuie să ruleze pe portul 3001 înainte de test-behavior.sh.
3. După modificări, repornește serverul (`pkill -f shop-mvp` + `cargo run -p shop-mvp &`).
4. Testele verifică conținut HTML, nu doar status code.
