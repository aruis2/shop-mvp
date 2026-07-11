---
title: "ISO 27001 — Information Security Management System"
description: "Politici și controale ISO 27001 pentru shop-mvp"
date: 2026-07-11
---

# ISO 27001 — ISMS Implementation

## Cuprins

1. [Ce este ISO 27001](#ce-este-iso-27001)
2. [Contextul organizației](#contextul-organizației)
3. [Leadership și politici](#leadership-și-politici)
4. [Planificare](#planificare)
5. [Suport](#suport)
6. [Operare](#operare)
7. [Evaluare și îmbunătățire](#evaluare-și-îmbunătățire)

## Ce este ISO 27001

ISO 27001 este standardul internațional pentru **Information Security Management Systems (ISMS)**.   
Publicat de ISO/IEC, definește cerințele pentru stabilirea, implementarea, menținerea și îmbunătățirea continuă a unui sistem de management al securității informației.

### Ciclul PDCA (Plan-Do-Check-Act)

```
Plan  →  Do  →  Check  →  Act
 │        │       │        │
 ├─Politici  ├─Implementare  ├─Audit  ├─Corecții
 ├─Risk      ├─Controale     ├─Review  ├─Îmbunătățiri
 ├─Scop      ├─Instruire     ├─Metrics │
 └─Obiective └─Operare       └─Reports  └─Update
```

### Anexa A — Controale (93 controale în 14 domenii)

| Domeniu | Controale | Status shop-mvp |
|---------|-----------|-----------------|
| A.5 — Politici de securitate | 2 | ✅ |
| A.6 — Organizare | 5 | ⚠️ Parțial |
| A.7 — Resurse umane | 3 | N/A |
| A.8 — Management active | 3 | ✅ |
| A.9 — Control acces | 4 | ✅ |
| A.10 — Criptografie | 2 | ⚠️ Parțial |
| A.11 — Securitate fizică | 3 | N/A |
| A.12 — Securitate operațională | 7 | ✅ |
| A.13 — Securitate comunicații | 3 | ✅ |
| A.14 — Achiziție sisteme | 3 | ✅ |
| A.15 — Relații furnizori | 2 | N/A |
| A.16 — Incident management | 2 | ⚠️ Parțial |
| A.17 — Business continuity | 3 | ⚠️ Parțial |
| A.18 — Compliance | 4 | ✅ |

## Contextul organizației

### Scopul ISMS

Protejarea confidențialității, integrității și disponibilității informațiilor procesate de platforma shop-mvp:
- Date personale ale utilizatorilor (nume, email, adresă, telefon)
- Date de plată (procesate prin Stripe — nu stocăm carduri)
- Date de business (produse, comenzi, prețuri)
- Codul sursă și configurațiile

### Stakeholders

- **Utilizatori finali**: confidențialitate + disponibilitate
- **Admini**: integritate + acces controlat
- **Autorități**: GDPR compliance (ANSPDCP)
- **Stripe**: procesator de plăți (certificat PCI DSS Level 1)

## Leadership și politici

### Politica de securitate a informației

```markdown
# Security Policy — Shop MVP

## Principii
1. **Confidențialitate**: datele utilizatorilor sunt accesibile doar
   cui trebuie
2. **Integritate**: datele nu pot fi modificate neautorizat
3. **Disponibilitate**: platforma e funcțională 99.9% din timp

## Măsuri tehnice
- Autentificare JWT cu expirare
- Capability-based access control
- HTTPS cu HSTS
- CSP anti-XSS
- Rate limiting anti-DoS
- Backup DB zilnic

## Roluri și responsabilități
- **Security Officer**: administratorul sistemului
- **Data Processor**: Stripe (plăți)
- **Data Controller**: proprietarul platformei
```

### Politica de acces

```markdown
# Access Control Policy

1. Principiul minimului privilegiu
   - Handlerele primesc doar capabilitățile necesare
   - Admin e un rol distinct, verificat explicit
2. Autentificare multi-factor (prin Stripe SCA)
3. Revocare acces la cerere (GDPR: ștergere cont)
```

## Planificare

### Risk Assessment

| Risc | Probabilitate | Impact | Măsură |
|------|--------------|--------|--------|
| SQL Injection | Scăzut | Foarte mare | SQLx query parameterizat |
| XSS | Scăzut | Mare | Tera auto-escape + CSP |
| CSRF | Scăzut | Mare | CSRF token pe POST-uri sensibile |
| Session hijack | Mediu | Mare | JWT + session timeout |
| Data breach | Scăzut | Foarte mare | Auth capability-based |
| DoS | Mediu | Mediu | Rate limiting + 2MB body limit |
| Stripe down | Scăzut | Mediu | RetryPayment cu backoff |

## Suport

### Competențe și instruire

- Rust (cunoașterea limbajului): avansat
- Securitate web: OWASP ASVS, CIS Controls
- Reglementări: GDPR, PSD2, PCI DSS
- DevOps: Docker, PostgreSQL, CI/CD

### Documentație

- **Cod**: auto-documentat cu comentarii în engleză și română
- **API**: documentat în `SPEC-CURL.md` și `TESTABIL-CU-CURL.md`
- **Arhitectură**: `PHILOSOPHY.md`, articole în `articles/`
- **Configurare**: `run-dev.sh`, `start.sh`, `compose.yml`

## Operare

### Managementul schimbărilor

1. Codul trece prin `cargo check` (0 erori, 0 warnings)
2. Teste Playwright rulează în CI
3. Review cod înainte de merge
4. Release: Docker image + cloudbuild.yaml

### Backup

- **DB**: zilnic la 3 AM prin `scripts/backup-db.sh`
- **Retention**: 30 de zile
- **Locație**: `/home/iuri/Desktop/2/backups/`

### Monitorizare

- Logging zilnic cu `tracing`
- Health check: `GET /health`
- Panic hook: backtrace în `logs/panic.log`
- Timp de răspuns per request

## Evaluare și îmbunătățire

### Audit intern

- `cargo check` — compilare fără erori
- `cargo clippy` — linting
- `cargo audit` — vulnerabilități dependențe

### Îmbunătățire continuă

1. Măsurare: număr de query-uri per request, timp de răspuns
2. Analiză: log-urile identifică blocaje
3. Corecție: optimizări hot path
4. Verificare: testare după fiecare schimbare
