# =============================================================================
# 🔴 Incident Response Playbook — Shop MVP
# =============================================================================
# CIS Control 17 + NIST CSF Respond
# =============================================================================

## Nivele de severitate

| Nivel | Culoare | Timp răspuns | Exemplu |
|-------|---------|--------------|---------|
| 🔴 **Critic** | Roșu | < 15 min | DB down, breach, Stripe key compromis |
| 🟠 **Ridicat** | Portocaliu | < 1h | Eroare 500 masivă, plăți blocate |
| 🟡 **Mediu** | Galben | < 4h | Login broken, pagină lentă |
| 🔵 **Scăzut** | Albastru | < 24h | CSS broken, link mort |

---

## 🔴 Critic — Playbook

### 1. DB down / Coruptă
```bash
# 1. Verifică
ssh -p 8022 u0_a481@192.168.1.4 "pg_isready -h localhost"

# 2. Restart
ssh -p 8022 u0_a481@192.168.1.4 "pg_ctlcluster 18 main restart"

# 3. Backup recovery
bash scripts/backup-db.sh  # forțează backup fresh

# 4. Verifică integritate
ssh -p 8022 u0_a481@192.168.1.4 "PGPASSWORD=123123 psql -h localhost -U postgres -d test -c 'SELECT count(*) FROM articles;'"
```

### 2. Stripe key compromis
```bash
# 1. Rotire cheie
bash scripts/secrets.sh rotate stripe

# 2. Verifică plăți suspecte
PGPASSWORD=123123 psql -h localhost -U postgres -d test -c "
  SELECT id, total_bani, payment_status, created_at
  FROM orders WHERE payment_status = 'paid'
  AND created_at > NOW() - INTERVAL '1 hour'
  ORDER BY created_at DESC;"

# 3. Notifică Stripe (dashboard)
```

### 3. Breach / Date leak
```bash
# 1. Salvează log-urile pentru investigație
cp logs/shop-mvp.log.2026-07-11 /tmp/incident-$(date +%s).log

# 2. Verifică accesări suspecte
grep -i "error\|401\|403\|429\|suspicious" logs/shop-mvp.log.2026-07-11 | tail -50

# 3. Verifică token-uri compromise
PGPASSWORD=123123 psql -h localhost -U postgres -d test -c "
  SELECT email, role, created_at FROM users
  WHERE created_at > NOW() - INTERVAL '1 day';"

# 4. Forțează logout la toți utilizatorii (schimbă JWT_SECRET)
bash scripts/secrets.sh rotate jwt
```

---

## 🟠 Ridicat — Playbook

### 1. Eroare 500 masivă
```bash
# 1. Verifică log-uri
tail -100 logs/shop-mvp.log.2026-07-11 | grep "500\|ERROR"

# 2. Verifică health
curl -f http://localhost:3001/health

# 3. Restart
pkill shop-mvp || true
cargo run -p shop-mvp &
```

### 2. Plăți blocate
```bash
# 1. Verifică Stripe status
curl -sI https://api.stripe.com | head -1

# 2. Verifică comenzi neprocesate
PGPASSWORD=123123 psql -h localhost -U postgres -d test -c "
  SELECT id, total_bani, payment_status
  FROM orders WHERE payment_status = 'pending'
  AND created_at > NOW() - INTERVAL '1 hour';"

# 3. Retry manual plăți eșuate (dacă există endpoint)
```

---

## 📞 Contacte

| Contact | Detalii |
|---------|---------|
| Dev (local) | Terminal + SSH S22 |
| Stripe | https://dashboard.stripe.com |
| DB (local) | `psql -h localhost -U postgres -d test` |
| DB (S22) | `ssh -p 8022 u0_a481@192.168.1.4` |

---

## 🔄 Post-mortem

După orice incident:
1. Salvează cronologia în `logs/incidents/`
2. Identifică cauza rădăcină
3. Adaugă test pentru a preveni recurența
4. Actualizează acest playbook
5. Actualizează STANDARDS.md dacă e cazul
