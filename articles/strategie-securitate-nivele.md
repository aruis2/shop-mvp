# Strategia securității pe nivele — De la `.env` la seL4

## Introducere

Securitatea nu e un produs, e un proces. Nu poți cumpăra „securitate enterprise" — o construiești
în funcție de nevoi, riscuri și buget. Acest articol prezintă o strategie pe 5 nivele,
de la un simplu `.env` până la verificare formală cu seL4.

Fiecare nivel rezolvă o problemă specifică și se justifică la un anumit volum de tranzacții.

---

## 1. Filosofia „cost vs risc"

Fiecare nivel de securitate are un cost și un beneficiu:

```
Cost lunar        │ Risc acoperit
──────────────────┼────────────────────────────
$0                │  0% (nu faci nimic)
$0.10             │ 80% (Secret Manager)
$5                │ 95% (Cloud HSM)
$50               │ 99% (Confidential VM)
$10,000+          │ 99.99% (seL4)
```

Regula de aur: **nu plăti pentru securitate de care nu ai nevoie încă**.

---

## 2. Nivel 0 — Dev / Early Stage

**Cost: 0 Lei**

### Ce facem
- `.env` cu chei secrete
- JWT cu secret hardcodat
- PostgreSQL cu user/pass local

### Riscuri
- Dacă serverul e spart, cheile sunt furate
- Dacă faci push la `.env` din greșeală → compromis total

### Cât rămânem aici
Până la primii clienți plătitori.

---

## 3. Nivel 1 — Producție mică (0 - $100K/lună)

**Cost: ~$0.10/lună**

### Ce facem
```
.env → Google Secret Manager

STRIPE_SECRET_KEY → Secret Manager
JWT_SECRET        → Secret Manager
```

### Cum funcționează

```rust
// Înainte
std::env::var("STRIPE_SECRET_KEY")

// După
secret_manager.get("STRIPE_SECRET_KEY").await
```

Cheile nu mai stau în fișiere locale, ci în API-ul securizat Google. Criptate
la stocare (AES-256) și în tranzit (TLS).

### Plus
- Rate limiting pe login/signup
- Input validation pe formuri
- Webhook Stripe (confirmare plată reală)

### Riscuri rămase
Un atacator care sparge serverul poate citi cheile din RAM (dump de memorie).

---

## 4. Nivel 2 — Growth ($100K - $1M/lună)

**Cost: ~$5-50/lună**

### Ce facem
```
Secret Manager → Google Cloud HSM + Confidential VMs
```

### HSM (Hardware Security Module)
Un cip dedicat care ține cheile și face signing **fără să le expună în RAM**.
Cheile nu părăsesc niciodată hardware-ul HSM-ului.

### Confidential VMs
RAM-ul mașinii virtuale e criptat hardware — nici Google nu poate citi
memoria procesului tău.

### Arhitectură

```
┌──────────────────────┐
│  Linux (app)         │
│                      │
│  Solicită:           │
│  "semnează JWT"      │
│  "fă plata"          │
└────────┬─────────────┘
         │ IPC
┌────────┴─────────────┐
│  HSM / TPM           │
│                      │
│  🔑 Cheile stau aici│
│  🔒 Nu ies din cip  │
└──────────────────────┘
```

### Riscuri rămase
Linux poate fi spart, dar atacatorul nu poate semna tokeni sau face plăți.

---

## 5. Nivel 3 — Enterprise ($1M - $10M/lună)

**Cost: ~$50-500/lună**

### Ce facem
```
HSM → Confidential VM + enclavă separată
```

Separăm fizic procesele sensibile de restul aplicației:
- **Linux**: aplicația (shop, API, frontend)
- **Enclavă**: signing, verificare, chei

### Arhitectură

```
┌──────────────────────┐
│  Linux (app)         │
│                      │
│  - Produse           │
│  - Coș               │
│  - Template-uri      │
│  - CRUD              │
└──────────────────────┘
         │
         │ IPC (comenzi simple)
         ▼
┌──────────────────────┐
│  Secure Enclave      │
│                      │
│  🔑 STRIPE_SECRET    │
│  🔑 JWT_SECRET       │
│                      │
│  Primește: "fă plata"│
│  Returnează: "ok"    │
│  Linux NU vede cheia │
└──────────────────────┘
```

### Câștig real
Chiar dacă atacatorul sparge Linux, **nu poate semna tokeni JWT**
și **nu poate face plăți** — cheile sunt în enclavă.

### Implementări posibile

| Soluție | Cost | Efort |
|---|---|---|
| Nitro Enclave (AWS) | ~$10/lună | 1 săptămână |
| Confidential VM (GCP) | ~$50/lună | 2 săptămâni |
| TPM bare-metal | ~$5/lună | 1 lună |

---

## 6. Nivel 4 — Ultra-Enterprise ($10M+/lună)

**Cost: zeci-sute de mii de dolari**

### Ce facem
```
Enclavă → seL4 (verificare formală)
```

### Diferența

| TPM / Nitro | seL4 |
|---|---|
| "Probabil că e sigur" | "Demonstrat matematic că e sigur" |
| Bug-uri posibile | Zero bug-uri (9K linii verificate) |
| Separare hardware | Separare hardware + verificare formală |
| Cost ~$50/lună | Cost ~$30K+ dezvoltare |

### Când alegi seL4
- Procesezi zeci de milioane de dolari lunar
- Un bug de securitate te poate scoate din business
- Ai audit extern care cere verificare formală
- Ești în domeniu bancar, militar sau medical

### Ce intră în securizată

```
┌──────────────────────────────────────┐
│       CAMERA SECURIZATĂ (seL4/TPM)   │
│                                      │
│  🔑 Chei secrete                     │
│     - STRIPE_SECRET_KEY              │
│     - JWT_SECRET                     │
│                                      │
│  ✍️ Operații criptografice           │
│     - Signare JWT                    │
│     - Verificare hash parole        │
│                                      │
│  📋 Audit log imutabil              │
│     - Cine a făcut refund           │
│     - Token-uri revocate            │
│                                      │
│  🔐 Verificări critice              │
│     - Webhook Stripe signature      │
│     - Validare token JWT            │
└──────────────────────────────────────┘
```

### Ce NU intră (rămâne în Linux)

```
│  🛒 Date de business                │
│     - Produse, prețuri, stoc        │
│     - Coșuri, comenzi               │
│     - Utilizatori (fără parole)     │
│                                      │
│  🎨 Prezentare                      │
│     - Template-uri Tera             │
│     - CSS, JS, imagini              │
│                                      │
│  🔄 Operații normale               │
│     - CRUD produse                  │
│     - Adăugat în coș                │
│     - Plafonare comenzi             │
└──────────────────────────────────────┘
```

---

## 7. Matricea decizională

```
Venit lunar       │ Soluție              │ Cost      │ Când treci
──────────────────┼──────────────────────┼───────────┼─────────────
$0 - $100K        │ Secret Manager       │ ~$0.10    │ ACUM
$100K - $1M       │ Cloud HSM            │ ~$5       │ 6 luni după
$1M - $10M        │ Confidential VM+TPM  │ ~$50      │ 1 an după
$10M+             │ seL4 / Nitro         │ $$$$      │ Caz special
```

---

## 8. Concluzie

| Nivel | Rezolvă |
|---|---|
| 0 | Nimic — rapid, fragil |
| 1 | Chei furate din fișiere |
| 2 | Chei furate din RAM |
| 3 | Server spart, dar chei safe |
| 4 | Verificare matematică |

> **Securitatea nu se cumpără, se construiește.**
> Fiecare nivel e justificat de volumul tău de tranzacții.
> Nu overthink-ui la nivelul 0 — ajunge când crești.

---

*Articol intern — strategie securitate Shop MVP*
*GitHub Copilot (DeepSeek V4 Flash) — 2026-07-09*
