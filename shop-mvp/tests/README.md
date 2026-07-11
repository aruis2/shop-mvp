# 🎭 Teste browser — Shop MVP (Playwright)

Teste automate care verifică HTMX, navigare, login, coșul — fără să dai click manual.

## Setup rapid

```bash
cd shop-mvp/tests

# 1. Instalează Playwright (prima dată)
npm install
npx playwright install chromium

# 2. Pornește PostgreSQL + server + seed
docker compose up -d                          # din rădăcina proiectului
bash ../scripts/seed-shop.sh                  # date de test
cargo run -p shop-mvp                         # serverul

# 3. Rulează testele (în alt terminal)
cd shop-mvp/tests
npx playwright test                           # headless
npx playwright test --ui                      # cu UI vizual
```

## Ce testează

| Test | Ce verifică |
|------|-----------|
| Pagina principală | Se încarcă `localhost:3001/` |
| Produse | Lista de produse + buton HTMX `+ Coș` |
| Login form | Are `hx-post` pe formular |
| Login greșit | Arată eroare `❌` după submit |
| Navigare HTMX | Click "Coș" → se încarcă fără refresh |
| Admin fără token | Redirect la login |
| Adăugare în coș | HTMX → coșul se actualizează |
| Ștergere din coș | HTMX → itemul dispare instant |
