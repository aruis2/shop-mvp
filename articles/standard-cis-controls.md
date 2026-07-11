---
title: "CIS Controls — 18 controale critice de securitate"
description: "Implementarea CIS (Center for Internet Security) Controls în shop-mvp"
date: 2026-07-11
---

# CIS Controls — Implementare

## Cuprins

1. [Introducere](#introducere)
2. [Inventarul controalelor](#inventarul-controalelor)
3. [Implementate în shop-mvp](#implementate-în-shop-mvp)
4. [Gap analysis](#gap-analysis)

## Introducere

CIS Controls (fost SANS Top 20) este un set de 18 controale prioritizate de securitate cibernetică, dezvoltat de Center for Internet Security. Fiecare control are implementări împărțite pe 3 nivele de maturitate (IG1, IG2, IG3).

## Inventarul controalelor

### Control 1: Inventory and Control of Hardware Assets
**Status**: N/A (aplicație web, nu infrastructură fizică)
**Notă**: Gestionat de Docker/compose.yml

### Control 2: Inventory and Control of Software Assets
**Status**: ✅ Implementat
- `Cargo.toml` + `Cargo.lock` pentru dependențe
- `Dockerfile` pentru imaginea de producție
- `cloudbuild.yaml` pentru CI/CD

### Control 3: Data Protection
**Status**: ✅ Implementat
- **GDPR**: Ștergere cont, export date, politică confidențialitate
- **PCI DSS**: Politică securitate, HTTPS, criptare
- SQLx query parameterizat (protecție SQL injection)

### Control 4: Secure Configuration of Enterprise Assets and Software
**Status**: ✅ Implementat
- Security headers: HSTS, CSP, X-Frame-Options, X-Content-Type-Options
- `.cargo/config.toml` cu mold + sccache
- `Dockerfile` minim (multi-stage build)
- Session timeout middleware

### Control 5: Account Management
**Status**: ✅ Implementat
- JWT-based authentication
- Account lockout (5 încercări / 15 minute)
- Logout handler
- Admin role verification

### Control 6: Access Control Management
**Status**: ✅ Implementat
- **Capability-based architecture** (seL4-style): fiecare handler primește doar ce-i trebuie
- `AuthState`, `ProductState`, `CartState`, `OrderState`, `AdminState` — tipuri separate
- Verificare admin în fiecare handler admin

### Control 7: Continuous Vulnerability Management
**Status**: ⚠️ Parțial
- ✅ `cargo audit` pentru dependențe
- ✅ Health check endpoint (`GET /health`)
- ✅ Panic hook cu logare
- ❌ Security.txt (vulnerability disclosure)
- ❌ Dependency scanning automatizat

### Control 8: Audit Log Management
**Status**: ✅ Implementat
- `tracing` cu logare zilnică (`logs/shop-mvp.log.YYYY-MM-DD`)
- Logging diferențiat: ERROR pentru 5xx, WARN pentru 4xx, INFO pentru 2xx
- Request timing cu UUID per request
- DB query counting
- `/admin/logs` — vizualizare log-uri în UI

### Control 9: Email and Web Browser Protections
**Status**: ✅ Implementat
- **CSP**: `default-src 'self'; script-src 'self' https://cdn.tailwindcss.com; style-src 'self' 'unsafe-inline'`
- **X-Frame-Options**: `DENY`
- **X-Content-Type-Options**: `nosniff`
- **Referrer-Policy**: `strict-origin-when-cross-origin`

### Control 10: Malware Defenses
**Status**: N/A (aplicație server, nu endpoint)

### Control 11: Data Recovery
**Status**: ✅ Implementat
- **Backup DB**: `scripts/backup-db.sh` (pg_dump + retention 30 zile)
- **Graceful shutdown**: handler pentru SIGTERM/SIGINT

### Control 12: Network Infrastructure Management
**Status**: ✅ Parțial
- `compose.yml` cu PostgreSQL containerizat
- Port explicit configurat (`PORT=3001`)

### Control 13: Network Monitoring and Defense
**Status**: N/A (gestionat de infrastructură)

### Control 14: Security Awareness and Skills Training
**Status**: N/A (organizațional)

### Control 15: Service Provider Management
**Status**: N/A (nu se aplică)

### Control 16: Application Software Security
**Status**: ✅ Implementat
- **OWASP ASVS Level 1**: Toate headerele de securitate
- **OWASP ASVS Level 2**: Session timeout, CSRF, idempotency, lockout, business limits
- **Rate limiting**: in-memory, 10 req/min/IP
- Body limit: 2MB max

### Control 17: Incident Response Management
**Status**: ⚠️ Parțial
- ✅ Panic hook cu backtrace în `logs/panic.log`
- ✅ Graceful shutdown
- ❌ Plan documentat de incident response

### Control 18: Penetration Testing
**Status**: ⚠️ Parțial
- ✅ Teste Playwright end-to-end (`shop-mvp/tests/shop.spec.ts`)
- ❌ Penetration testing policy documentată
