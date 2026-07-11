# 📋 STANDARDS.md — Standards & Compliance

> Toate standardele de securitate, confidențialitate și calitate pe care le urmărim, le-am implementat, sau le vom implementa.

---

## 🏗️ Full Stack — de la BIOS la cod

> Urmărim standarde pentru **fiecare nivel** al stivei, indiferent unde e deployată aplicația.
> Cînd știm mediul exact, aplicăm configurarea specifică.

```
┌──────────────────────────────────────────────────────────┐
│                    APLICAȚIE                              │
│  OWASP ASVS · PCI DSS · WCAG · API Top 10 · GDPR · eIDAS │
│  Headere HTTP · TLS · Cookie · CSRF · Rate limit · JWT   │
├──────────────────────────────────────────────────────────┤
│                    LIMBAJ (Rust)                          │
│  Edition 2024 · clippy · 0 unsafe · async · mold+sccache │
│  Capability-based · cargo audit · cross-compile          │
├──────────────────────────────────────────────────────────┤
│                    BAZĂ DE DATE (PostgreSQL)              │
│  SQLx parametrizat · Indexuri GIN · pgvector · Migrations│
│  FTS română · Backup · Connection pool · PgBouncer       │
├──────────────────────────────────────────────────────────┤
│                    CONTAINER (Docker)                     │
│  Docker Bench Security · Trivy · CIS Docker · Multi-stage │
│  User namespace · Read-only root · No privilege           │
├──────────────────────────────────────────────────────────┤
│                    SISTEM DE OPERARE (Linux)              │
│  CIS Benchmarks Linux · AppArmor/SELinux · Auditd        │
│  Kernel lockdown · sysctl hardening · PAM · password poli │
│  LUKS (encryption) · AIDE (integrity) · Fail2ban         │
├──────────────────────────────────────────────────────────┤
│                    REȚEA                                   │
│  Firewall (iptables/nftables) · WireGuard VPN · DNS-over-TLS│
│  Network segmentation · Port knocking · IDS/IPS (Snort)  │
├──────────────────────────────────────────────────────────┤
│                    BOOT + FIRMWARE                        │
│  UEFI Secure Boot · TPM 2.0 · Measured Boot · GRUB signat │
│  Intel Boot Guard · BIOS password · Secure erase          │
├──────────────────────────────────────────────────────────┤
│                    HARDWARE                               │
│  TPM 2.0 (Trusted Platform Module) · HSM · YubiKey/Nitro │
│  NIST SP 800-147 (BIOS) · TCG standards · FIPS 140-3     │
└──────────────────────────────────────────────────────────┘
```

### Nivelul 1 — Hardware

| # | Standard | Descriere | Cînd se aplică |
|---|----------|-----------|----------------|
| H1 | **TPM 2.0** (Trusted Platform Module, ISO/IEC 11889) | Cip criptografic pentru stocare chei, atestare hardware | Orice server fizic |
| H2 | **TCG Standards** (Trusted Computing Group) | Specificații pentru hardware trusted | Server fizic |
| H3 | **NIST SP 800-147** — BIOS Protection Guidelines | Protecție și integritate BIOS/UEFI | Server fizic |
| H4 | **NIST SP 800-155** — BIOS Integrity Measurement | Măsurarea integrității BIOS | Server fizic |
| H5 | **FIPS 140-3** (Level 2+) | Validare module criptografice hardware | Doar dacă e cerut |
| H6 | **HSM** (Hardware Security Module) | Stocare chei criptografice în hardware dedicat | Stripe key, JWT signing |
| H7 | **YubiKey / Nitrokey** | Autentificare hardware 2FA | Acces admin, SSH |
| H8 | **Secure Erase** (NIST SP 800-88) | Ștergere sigură date de pe disk | La decommissioning |
| H9 | **ECC / AES-NI** | Instrucțiuni criptografice hardware | Orice procesor modern |

### Nivelul 2 — Firmware + Boot

| # | Standard | Descriere | Cînd se aplică |
|---|----------|-----------|----------------|
| F1 | **UEFI Secure Boot** | Boot doar cu firmware semnat | Orice sistem |
| F2 | **Intel Boot Guard** | Verificare integritate boot ROM | Intel-based |
| F3 | **AMD Platform Secure Boot** | Verificare integritate boot ROM | AMD-based |
| F4 | **Measured Boot** (TPM) | Măsurarea fiecărui pas al boot-ului | Server/laptop |
| F5 | **GRUB signing** | Bootloader semnat criptografic | Linux |
| F6 | **Kernel lockdown** (EFI) | Kernel semnat, restricționat | Producție |
| F7 | **BIOS/UEFI password** | Parolă pentru acces BIOS | Orice server fizic |
| F8 | **Intel ME / AMD PSP** | Management engine — dezactivare parțială | Secure environment |

### Nivelul 3 — Sistem de Operare

| # | Standard | Descriere | Cînd se aplică |
|---|----------|-----------|----------------|
| OS1 | **CIS Benchmark for Linux** | 100+ reguli de hardening | Orice Linux |
| OS2 | **NIST SP 800-123** — Server Security | Ghid securizare server | Linux server |
| OS3 | **NIST SP 800-53** — Security Controls | Controale de securitate detaliate | Federal/Enterprise |
| OS4 | **AppArmor / SELinux** | Mandatory Access Control | Linux |
| OS5 | **sysctl hardening** | Kernel parameters sigure: `kernel.kptr_restrict=2`, `net.ipv4.conf.all.rp_filter=1`, `kernel.dmesg_restrict=1` | Linux |
| OS6 | **pam_passwdqc / pwquality** | Politici parole: lungime, complexitate, expirare | Login |
| OS7 | **auditd** | Audit logging: `ausearch`, `aureport` | Orice Linux |
| OS8 | **AIDE / Tripwire** | File integrity monitoring | Server producție |
| OS9 | **LUKS / dm-crypt** | Full disk encryption | Laptop, server fizic |
| OS10 | **Kernel Live Patching** | Patch-uri kernel fără restart | Producție |
| OS11 | **Linux kernel lockdown** | Restricționare acces kernel (/dev/mem, kprobes) | Producție |
| OS12 | **umask 027** | Permisiuni implicite restrictive | Orice |
| OS13 | **Core dumps restriction** | `fs.suid_dumpable=0`, `core_pattern` | Orice |

### Nivelul 4 — Rețea

| # | Standard | Descriere | Cînd se aplică |
|---|----------|-----------|----------------|
| N1 | **Firewall (iptables/nftables)** | Reguli intrare/ieșire: doar porturi necesare | Orice server |
| N2 | **WireGuard** | VPN point-to-point criptat | S22 ↔ desktop |
| N3 | **NIST SP 800-41** — Firewall Guidelines | Ghid configurare firewall | Enterprise |
| N4 | **NIST SP 800-94** — IDS/IPS | Intrusion Detection/Prevention | Producție |
| N5 | **Network segmentation** | Rețele separate: DB, app, public | Cloud/on-prem |
| N6 | **Port knocking** | Ascundere porturi SSH | SSH public |
| N7 | **DNS-over-TLS** (RFC 7858) | Rezolvare DNS criptată | Orice |
| N8 | **DNSSEC** (RFC 4033-4035) | Integritate DNS | Domeniu public |
| N9 | **mTLS** | TLS mutual între servicii | Microservicii |

### Nivelul 5 — Container

| # | Standard | Descriere | Cînd se aplică |
|---|----------|-----------|----------------|
| C1 | **CIS Docker Benchmark** | 100+ reguli hardening Docker | Docker |
| C2 | **NIST SP 800-190** — Container Security | Ghid securizare containere | Docker/K8s |
| C3 | **Trivy / Grype / Clair** | Vulnerability scanning imagini | CI/CD |
| C4 | **Docker multi-stage build** | Build-uri minimale, fără tooluri de dev | Docker |
| C5 | **Read-only root filesystem** | Container rootfs read-only | Docker |
| C6 | **No privileged containers** | Fără `--privileged` | Docker |
| C7 | **User namespace remapping** | Root în container ≠ root pe host | Docker |
| C8 | **Resource limits** | `--memory`, `--cpus` | Docker |
| C9 | **Image signing** (Cosign) | Semnare imagini Docker | Producție |
| C10 | **Docker Content Trust** | Verificare semnătură imagini | Producție |

### Nivelul 6 — Aplicație (deja implementat)

> Vezi secțiunea [🟢 Implementate](#🟢-implementate-în-cod) mai jos.

### Matrice cross-level

| Amenințare | Hardware | Boot | OS | Rețea | Container | App |
|------------|----------|------|----|-------|-----------|-----|
| **Physical access** | TPM, Secure Erase | Secure Boot | LUKS | — | — | — |
| **Rootkit** | Boot Guard | Measured Boot | AIDE, lockdown | — | — | — |
| **Network attack** | — | — | sysctl, auditd | Firewall, IDS | — | WAF |
| **Container escape** | — | — | AppArmor | — | User ns, read-only | — |
| **SQL injection** | — | — | — | — | — | SQLx param ✅ |
| **Credential leak** | HSM | — | PAM | WireGuard | — | Secrets.sh |

---

## 📌 Ce mai lipsește (goluri identificate)

> Chiar și cu acoperirea vastă, există niște standarde pe care încă nu le urmărim.
> Le adăugăm aici ca să știm că există.

### Management / Procese

| # | Standard | Domeniu | De ce lipsește |
|---|----------|---------|----------------|
| M1 | **NIST SP 800-61** — Incident Handling Guide | Incident response | Nu avem încă un proces formal de handling |
| M2 | **NIST SP 800-115** — Penetration Testing | Testare | Nu avem penetration testing programat |
| M3 | **NIST SP 800-37** — Risk Management Framework (RMF) | Risk management | Proces formal de risk management |
| M4 | **ITIL 4** — IT Service Management | ITSM | Doar dacă avem echipă IT dedicată |
| M5 | **ISO 22301** — Business Continuity | BCM | Business continuity plan formal |
| M6 | **COBIT 2019** — IT Governance | Guvernanță | Doar pentru audit intern avansat |

### Cloud-specific

| # | Standard | Domeniu | De ce lipsește |
|---|----------|---------|----------------|
| C11 | **AWS Well-Architected Framework** | Cloud AWS | Nu știm încă unde facem deploy |
| C12 | **GCP Security Foundations** | Cloud GCP | Nu știm încă unde facem deploy |
| C13 | **Azure Security Benchmark** | Cloud Azure | Nu știm încă unde facem deploy |
| C14 | **Kubernetes CIS Benchmark** | K8s | Nu știm dacă folosim K8s |
| C15 | **Pod Security Standards** | K8s | Nu știm dacă folosim K8s |
| C16 | **OCI Runtime Specification** | Container runtime | Doar dacă schimbăm runtime-ul |

### Tehnologii specifice

| # | Standard | Domeniu | De ce lipsește |
|---|----------|---------|----------------|
| T1 | **OWASP LLM Top 10** | AI/ML Security | Nu avem încă LLM în aplicație |
| T2 | **NIST AI RMF** (AI Risk Management) | AI/ML | Nu avem încă AI în producție |
| T3 | **NIST SP 800-213** — IoT Security | IoT | Nu avem device-uri IoT |
| T4 | **OWASP Mobile Top 10** | Mobile | Nu avem aplicație mobilă |
| T5 | **SBOM** (Software Bill of Materials) | Supply chain | CycloneDX / SPDX — de adăugat în CI |

### Kernel / OS avansat

| # | Standard | Domeniu | De ce lipsește |
|---|----------|---------|----------------|
| K1 | **KSPP** (Kernel Self Protection Project) | Kernel | Set de patch-uri de securitate kernel |
| K2 | **seccomp-bpf** profiles | Container | Profile seccomp implicite pentru containere |
| K3 | **USBGuard** | USB device control | Control device-uri USB |
| K4 | **BPF hardening** | Kernel | Restricționare eBPF |
| K5 | **ASLR sensitivity** | Kernel | îmbunătățire randomizare memorie |
| K6 | **NIST SP 800-90B** — RNG Validation | Hardware RNG | Validare generator numere aleatoare hardware |

### Rețea avansat

| # | Standard | Domeniu | De ce lipsește |
|---|----------|---------|----------------|
| R1 | **RPKI / BGPSec** | BGP security | Doar dacă avem propriul AS |
| R2 | **MACsec** (802.1AE) | Layer 2 encryption | Doar în rețea locală |
| R3 | **NetFlow / IPFIX** | Network monitoring | Monitorizare trafic rețea |
| R4 | **DDoS protection** | DDoS | Scrubber, rate limiting la edge |

---

## 📊 Total standarde urmărite

| Categorie | Număr |
|-----------|-------|
| Hardware | 9 |
| Firmware + Boot | 8 |
| Sistem de Operare | 13 + 6 (avansat) |
| Rețea | 9 + 4 (avansat) |
| Container | 10 + 5 (K8s) |
| Bază de date | 16 |
| Aplicație | 18 |
| Protocoale HTTP | 6 |
| Rust | 16 |
| DevOps | 10 |
| Calitate | 4 |
| Management / Procese | 6 (nou) |
| Cloud-specific | 6 (nou) |
| Tehnologii specifice | 5 (nou) |
| **Total** | **~120+** |

## 🟢 Implementate (în cod)

### Standarde de securitate și confidențialitate

| # | Standard | Acronim | Domeniu | Implementat |
|---|----------|---------|---------|-------------|
| 1 | **OWASP ASVS Level 1** — Application Security Verification Standard | ASVS L1 | Securitate web | ✅ Acoperit implicit de L2 — V9 (HSTS), V8.3 (Cache-Control), V10 (CSP), V4 (X-Frame-Options), V8 (X-Content-Type-Options), V9 (Referrer-Policy) |
| 2 | **OWASP ASVS Level 2** — Verificare avansată (include L1) | ASVS L2 | Securitate web | ✅ V3.3.1 (Session timeout), V3.4 (Account lockout 5/15min), CSRF tokens, V2.5 (Idempotency), Business logic limits (20 items/10.000 lei) |
| 3 | **GDPR** — General Data Protection Regulation (UE) | GDPR | Confidențialitate | ✅ Art. 17 (Ștergere cont), Art. 20 (Export date), Art. 13 (Politică confidențialitate — `/privacy`) |
| 4 | **PCI DSS** — Payment Card Industry Data Security Standard | PCI DSS | Plăți | ✅ Politică securitate (`/security`), HTTPS+TLS 1.2+, HSTS, Stripe (PCI DSS Level 1 compliant) |
| 5 | **NIST CSF** — Cybersecurity Framework | NIST CSF | Securitate cibernetică | ✅ Recover (Backup DB automat — `scripts/backup-db.sh`), Identify/Protect/Detect/Respond (parțial) |
| 6 | **PSD2 / SCA** — Payment Services Directive 2 + Strong Customer Authentication | PSD2/SCA | Plăți (UE) | ✅ Stripe Checkout cu 3D Secure, webhook Stripe (`POST /stripe/webhook`), idempotency |
| 7 | **CIS Controls** v8 — Center for Internet Security Critical Security Controls | CIS | Securitate | ✅ Control 2 (Software inventory), 3 (Data protection), 4 (Secure config), 5 (Account mgmt), 6 (Access control), 7 (Vulnerability disclosure — `/.well-known/security.txt`), 8 (Audit log), 9 (Browser protections), 11 (Data recovery), 16 (App security) |
| 8 | **WCAG 2.1 Level AA** — Web Content Accessibility Guidelines | WCAG | Accesibilitate | ✅ Skip-to-content, ARIA roles/labels, focus visible, `prefers-reduced-motion`, contrast ≥ 4.5:1, keyboard navigation |
| 9 | **OWASP API Top 10** — API Security Risks | API Top 10 | Securitate API | ✅ API1 (BOLA — capability-based), API2 (JWT+lockout), API3 (Compile-time garanție), API4 (Rate limit), API5 (Admin check), API6 (Idempotency), API7 (SSRF), API8 (Headers), API10 (Retry+timeout) |
| 10 | **ISO 27001** — Information Security Management System | ISO 27001 | Management | ✅ Politici ISMS, risk assessment, SoA, ciclu PDCA documentat |
| 11 | **eIDAS** — Electronic IDentification, Authentication and Trust Services (UE) | eIDAS | Identitate digitală | ✅ Model factură cu hash digital, nivele de asigurare (scăzut/substanțial/înalt) |
| 12 | **HTTP Security Headers** — HSTS (RFC 6797), CSP (W3C), X-Frame-Options (RFC 7034), X-Content-Type-Options, Referrer-Policy (W3C), Cache-Control (RFC 7234) | HTTP | Protocoale web | ✅ HSTS (`max-age=31536000; includeSubDomains`), CSP (`default-src 'self'`), X-Frame-Options (`DENY`), X-Content-Type-Options (`nosniff`), Referrer-Policy (`strict-origin-when-cross-origin`), Cache-Control (`no-store` pe rute sensibile) |
| 13 | **TLS 1.2+** — Transport Layer Security (RFC 5246 / RFC 8446) | TLS | Criptografie | ✅ HTTPS implicit, HSTS forțează TLS |
| 14 | **HTTP Strict Transport Security** — RFC 6797 | HSTS | Protocoale web | ✅ `max-age=31536000; includeSubDomains` |
| 15 | **Cookie Security** — RFC 6265 | Cookie | Protocoale web | ✅ HttpOnly, SameSite=Strict/Lax, Path=/ |
| 16 | **HTTP/1.1** — Message Syntax and Routing (RFC 7230-7235) | HTTP/1.1 | Protocoale web | ✅ Status codes corecți (302, 401, 403, 429, 500), metode standard (GET, POST), headere standard |
| 17 | **CORS** — Cross-Origin Resource Sharing (Fetch standard) | CORS | Protocoale web | ✅ `Access-Control-Allow-Origin: *` pe security.txt, Same-Origin policy implicit |
| 18 | **RFC 9116** — security.txt | security.txt | Vulnerability disclosure | ✅ `GET /.well-known/security.txt` |

## 🟡 Implementate parțial (în curs)

| # | Standard | Implementat | Ce lipsește |
|---|----------|-------------|-------------|
| 19 | **NIST CSF** (complet) | Identify ✅, Protect ✅, Detect ✅, Respond ⚠️, Recover ✅ | Incident response plan documentat |
| 20 | **CIS Control 7** — Vulnerability Management | ✅ security.txt | Dependency scanning automatizat (`cargo audit` în CI) |
| 21 | **CIS Control 17** — Incident Response | ✅ Panic hook, graceful shutdown | Plan documentat de incident response |
| 22 | **CIS Control 18** — Penetration Testing | ✅ Playwright E2E tests | Politică de penetration testing documentată |
| 23 | **OWASP API Top 10 — API9** | ✅ Rute listate la startup | OpenAPI/Swagger documentation |
| 24 | **WCAG 2.1 Level AAA** | Level AA complet | Level AAA (contrast 7:1, sign language, etc.) |

---

## 🔵 Urmărite (planificate pentru viitor)

### Standarde de securitate

| # | Standard | Descriere | Prioritate | Motiv |
|---|----------|-----------|------------|-------|
| 18 | **OWASP Top 10** — 2021 | Cele mai critice 10 riscuri web | ⭐ Foarte mare | Acoperit implicit de ASVS, dar monitorizat separat |
| 19 | **OWASP Mobile Top 10** | Securitate aplicații mobile | 🟡 Medie | Dacă apare o aplicație mobilă |
| 20 | **SOC 2 Type I + II** | Service Organization Control | 🟡 Medie | Dacă hostăm servicii pentru terți |
| 21 | **FedRAMP** | US Government Cloud Security | 🔴 Scăzută | Doar dacă intrăm pe piața US Gov |
| 22 | **BSI C5** | Cloud Computing Compliance (Germania) | 🔴 Scăzută | Doar dacă hostăm în Germania |
| 23 | **CSA STAR** | Cloud Security Alliance | 🔴 Scăzută | Complementar ISO 27001 |
| 24 | **FIPS 140-3** | Cryptographic Module Validation | 🔴 Scăzută | Doar dacă avem nevoie de criptografie certificată |
| 25 | **COBIT 2019** | IT Governance Framework | 🔴 Scăzută | Doar pentru audit intern avansat |

### Confidențialitate

| # | Standard | Descriere | Prioritate | Motiv |
|---|----------|-----------|------------|-------|
| 26 | **CCPA/CPRA** | California Consumer Privacy Act | 🟡 Medie | Dacă avem utilizatori în California |
| 27 | **LGPD** | Lei Geral de Proteção de Dados (Brazilia) | 🔴 Scăzută | Dacă avem utilizatori în Brazilia |
| 28 | **PIPEDA** | Personal Information Protection (Canada) | 🔴 Scăzută | Dacă avem utilizatori în Canada |
| 29 | **ISO 27701** | Privacy Information Management | 🟡 Medie | Extindere ISO 27001 pentru confidențialitate |
| 30 | **ISO 27018** | Cloud Privacy — PII | 🔴 Scăzută | Doar dacă hostăm în cloud public |
| 31 | **ePrivacy Directive** | Directiva confidențialitate comunicații (UE) | 🟡 Medie | Cookie-uri, tracking, marketing |

### Accesibilitate

| # | Standard | Descriere | Prioritate | Motiv |
|---|----------|-----------|------------|-------|
| 32 | **WCAG 2.2** | Versiunea curentă (2023) | ⭐ Foarte mare | Upgrade de la 2.1 la 2.2 |
| 33 | **WCAG 3.0** | Viitoarea versiune (în lucru) | 🟡 Medie | Cînd e finalizată de W3C |
| 34 | **EN 301 549** | European standard for accessibility | ⭐ Foarte mare | Cerință legală UE (European Accessibility Act) |
| 35 | **Section 508** | US federal accessibility | 🔴 Scăzută | Doar pentru piața US federală |

### Plăți și financiar

| # | Standard | Descriere | Prioritate | Motiv |
|---|----------|-----------|------------|-------|
| 36 | **PSD3** | Payment Services Directive 3 (2025+) | 🟡 Medie | Upgrade de la PSD2 |
| 37 | **SEPA** | Single Euro Payments Area | 🟡 Medie | Dacă adăugăm transferuri bancare directe |
| 38 | **3D Secure 2.x** | Authentication protocol | ✅ Deja | Acoperit prin Stripe |
| 39 | **ISO 20022** | Financial messaging standard | 🔴 Scăzută | Doar pentru integrare bancară directă |
| 40 | **FATF Recommendations** | Anti-Money Laundering | 🔴 Scăzută | Dacă volumul de plăți crește semnificativ |

### DevOps și infrastructură

| # | Standard | Descriere | Prioritate | Motiv |
|---|----------|-----------|------------|-------|
| 41 | **CIS Benchmarks** | Hardening guides (Linux, Docker, PostgreSQL) | ⭐ Foarte mare | Securizare infrastructură |
| 42 | **SLSA** | Supply-chain Levels for Software Artifacts | 🟡 Medie | Integritatea lanțului de aprovizionare software |
| 43 | **OpenSSF Scorecard** | Securitate open-source | 🟡 Medie | Evaluare securitate dependențe |
| 44 | **Docker Bench Security** | Docker hardening | ⭐ Foarte mare | Securizare containere |
| 45 | **Trivy/Grype scanning** | Vulnerability scanning în CI | ⭐ Foarte mare | Scanare imagini Docker + dependencies |
| 46 | **Sigstore** | Software signing | 🟡 Medie | Semnare artifacte release |
| 47 | **Secrets management** | Criptare `.env` cu `age` | ✅ Deja | `scripts/secrets.sh` — encrypt/decrypt/rotate |
| 48 | **Incident Response Playbook** | Playbook pentru incidente | ✅ Deja | `INCIDENT-RESPONSE.md` — 3 niveluri, playbook per scenariu |
| 49 | **CI/CD Security** | `cargo audit` + clippy + fmt în CI | ✅ Deja | `.github/workflows/security.yml` |
| 50 | **`.env.example`** | Template env fără secrete | ✅ Deja | `.env.example` — documentat complet |

### Rust — limbaj și tooling

| # | Standard | Descriere | Prioritate | Motiv |
|---|----------|-----------|------------|-------|
| 47 | **Rust Edition 2024** | Ediția curentă a limbajului | ✅ Deja | `edition = "2024"` în `Cargo.toml` |
| 48 | **Rust API Guidelines** | Convenții de numire, trait-uri, macro-uri | ✅ Deja | Tipuri `impl Trait`, `From`/`Into`, numire `snake_case` |
| 49 | **`#![deny(unsafe_code)]`** | Interzicere cod unsafe | ✅ Deja | 0 `unsafe` în tot proiectul |
| 50 | **`cargo clippy`** | Linting Rust | ✅ Deja | 0 warnings |
| 51 | **`cargo fmt`** | Formatare automată | ✅ Deja | `.rustfmt.toml` |
| 52 | **`cargo audit`** | Scanare vulnerabilități dependențe | 🟡 Parțial | De adăugat în CI |
| 53 | **mold linker** | Linker ultra-rapid | ✅ Deja | `.cargo/config.toml`: `rustflags = ["-C", "link-arg=-fuse-ld=mold"]` |
| 54 | **sccache** | Caching compilare distribuit | ✅ Deja | `build.rustc-wrapper = "sccache"` |
| 55 | **Rust 2021 -> 2024 migration** | Migrare la ediția curentă | ✅ Deja | `resolver = "3"` în workspace |
| 56 | **Module per feature** | Organizare cod pe domenii | ✅ Deja | 9 crate-uri, fiecare cu un singur scop |
| 57 | **async-first** | Programare asincronă cu tokio | ✅ Deja | Axum + sqlx + tokio |
| 58 | **Capability-based architecture** | seL4-style: fiecare handler vede doar ce-i trebuie | ✅ Deja | `AuthState`, `ProductState`, etc. |
| 59 | **No macros** | Fără macro-uri sqlx (query! verifica la runtime) | ✅ Deja | `query_as::<_, T>` pattern |
| 60 | **Cross-compilation** | Build pentru ARM (S22), x86 (desktop), Cloud Run | ✅ Deja | Script `build-cross.sh` |
| 61 | **Rustdoc** | Documentație cod | 🟡 Parțial | Comentarii în engleză+română, fără doc testuri |
| 62 | **Property-based testing** | Testare cu generare automată date | 🔴 Scăzută | `proptest` / `quickcheck` |
| 63 | **Rust gap conștientizat** | Bug-uri pe care Rust NU le prinde la compilare (IDOR, state machine, race conditions, business logic) — vezi PHILOSOPHY #15 | 🟡 Medie | `LogicFactory::verify_*()` + capability-based state + tranzacții SQL |

### PostgreSQL — baze de date

| # | Standard | Descriere | Prioritate | Motiv |
|---|----------|-----------|------------|-------|
| 63 | **PostgreSQL 18** | Versiunea curentă | ✅ Deja | `compose.yml` + pgvector |
| 64 | **SQLx query parameterized** | Prevenire SQL injection | ✅ Deja | `$1, $2, ...` binding peste tot |
| 65 | **Migrations** | Schema versionată | ✅ Deja | `shop-mvp/migrations/001_shop_schema.sql` |
| 66 | **Indexing** | Indexuri pentru performanță | ✅ Deja | `idx_products_slug`, GIN pe tags, GIN pe category_path, GIN full-text search |
| 67 | **pgvector** | Căutare semantică | ✅ Deja | `vector(768)` embeddings, index IVFFlat |
| 68 | **Connection pooling** | Pool de conexiuni | ✅ Deja | `PgPool::connect()` |
| 69 | **Backup automat** | Backup DB programat | ✅ Deja | `scripts/backup-db.sh` (zilnic, 30 zile retention) |
| 70 | **Docker PostgreSQL** | PostgreSQL containerizat | ✅ Deja | `compose.yml` cu configurare optimizată |
| 71 | **Query optimization** | Configurare DB performantă | ✅ Deja | `shared_buffers=512MB`, `random_page_cost=1.1` |
| 72 | **Full-text search** | Căutare full-text în română | ✅ Deja | `to_tsvector('romanian', ...)` |
| 73 | **Health check** | Verificare conexiune DB | ✅ Deja | `GET /health` cu `SELECT 1` |
| 74 | **Logging query timing** | Monitorizare performanță query-uri | ✅ Deja | `tracing` + `DB_QUERY_COUNT` |
| 75 | **Stored procedures** | Logica DB în server | 🔴 Scăzută | Totul e în Rust, nu în SQL |
| 76 | **Read replicas** | Replici pentru citire | 🔴 Scăzută | Doar la scalare |
| 77 | **Connection pooling (PgBouncer)** | Pool extern pentru conexiuni multe | 🔴 Scăzută | Doar la sute de conexiuni simultane |
| 78 | **Migrate to Cloud SQL** | Trecere la PostgreSQL cloud (GCP) | 🟡 Medie | Script `migrate-to-cloud-sql.sh` existent |

### Protocoale HTTP și rețea

| # | Standard | Descriere | Prioritate | Motiv |
|---|----------|-----------|------------|-------|
| 47 | **HTTP/2** — RFC 7540 / RFC 9113 | Protocol îmbunătățit (multiplexare, compresie headere) | ⭐ Foarte mare | Performanță, deja suportat de Axum/Tokio |
| 48 | **HTTP/3** — RFC 9114 (QUIC) | Protocol peste UDP, zero RTT handshake | 🟡 Medie | Performanță pe mobile/latency mare |
| 49 | **OCSP Stapling** — RFC 6961 | Verificare certificat TLS fără conexiune separată | 🟡 Medie | Performanță handshake TLS |
| 50 | **Certificate Transparency** — RFC 9162 | Monitorizare certificate SSL | 🟡 Medie | Securitate PKI |
| 51 | **DNS-over-HTTPS** — RFC 8484 | Rezolvare DNS criptată | 🔴 Scăzută | Doar pentru clienți |
| 52 | **IPv6** | Protocol rețea versiunea 6 | 🟡 Medie | Compatibilitate rețea |

### Calitate și testare

| # | Standard | Descriere | Prioritate | Motiv |
|---|----------|-----------|------------|-------|
| 47 | **ISO 25010** | Software Quality Model | 🟡 Medie | Evaluare calitate cod |
| 48 | **OWASP ASVS Level 3** | Verificare avansată (la nivel de cod sursă) | 🟡 Medie | Următorul nivel după L2 |
| 49 | **OWASP SAMM** | Software Assurance Maturity Model | 🟡 Medie | Maturitatea proceselor de securitate |
| 50 | **BSIMM** | Building Security In Maturity Model | 🔴 Scăzută | Alternativă la SAMM |

---

## 📊 Matrice de conformitate

### Cerințe legale (UE)

| Standard | Obligatoriu | Implementat | Termen |
|----------|-------------|-------------|--------|
| GDPR | Da, din 2018 | ✅ | Imediat |
| ePrivacy Directive | Da | ⚠️ Parțial | 2026 |
| PSD2/SCA | Da, din 2019 | ✅ | Imediat |
| European Accessibility Act | Da, din 2025 | ✅ WCAG 2.1 AA | 2026 |
| eIDAS 2.0 | Da, din 2026 | ⚠️ Parțial | 2026-2027 |
| PSD3 | Viitor | ⬜ | 2027+ |

### Certificări voluntare

| Certificare | Beneficiu | Efort | Planificat |
|-------------|-----------|-------|------------|
| ISO 27001 | Credibilitate internațională | Mare | 2027 |
| SOC 2 | Încredere cloud | Mediu | 2027 |
| PCI DSS (Stripe) | Deja prin Stripe | Zero | ✅ |
| CIS Controls | Securitate practică | Mic | ✅ |

---

## 🔗 Articole și resurse asociate

Toate standardele au articole detaliate în `articles/` și în knowledge base:

### Standarde de securitate
- `articles/standard-owasp-asvs-level-2.md`
- `articles/standard-gdpr-web.md`
- `articles/standard-pci-dss.md`
- `articles/standard-nist-csf.md`
- `articles/standard-psd2-sca.md`
- `articles/standard-cis-controls.md`
- `articles/standard-iso-27001.md`
- `articles/standard-eidas.md`
- `articles/standard-wcag-21.md`
- `articles/standard-owasp-api-top-10.md`

### Rust
- `articles/cross-compilare.md` — Cross-compilare Rust
- `articles/mold-sccache-advanced.md` — mold linker + sccache
- `libs/` — Cele 9 crate-uri, fiecare cu documentație

### PostgreSQL
- `libs/cache/` — PgCache cu TTL
- `libs/rust-knowledge-base/` — Căutare full-text + semantică
- `shop-mvp/migrations/001_shop_schema.sql` — Schema DB
- `articles/pgvector-android-termux.md` — pgvector pe Android/Termux

### Arhitectură și DevOps
- `articles/arhitectura-lego-hotpath.md` — LEGO modules + Hot Path
- `articles/arhitectura-dual-project-myapp-shop-mvp.md` — Dual project ecosystem
- `articles/hot-path.md` — Hot Path optimization
- `articles/cross-compilare.md` — Cross-compilare
- `articles/mold-sccache-advanced.md` — mold + sccache
- `articles/webassembly-prod.md` — WebAssembly
- `articles/ubuntu-dev-optimizare.md` — Ubuntu dev environment

### Securitate (articole generale)
- `articles/securitate-informatica.md` — Securitate informatică
- `articles/strategie-securitate-nivele.md` — Nivele de securitate
- `articles/ce-este-enterprise.md` — Enterprise features
- `articles/bucla-infinita-de-redirect-care-nu-era-javascript.md` — Debugging redirect
- `articles/debugging-infinite-redirect-loop.md` — Debugging redirect (en)
- `articles/frontend-debugging-journey.md` — Frontend debugging

---

## 🏆 Goal — nivelul maxim per standard

> Urmărim să atingem nivelul cel mai avansat al fiecărui standard.  
> Tabelul arată unde sîntem acum vs. unde putem ajunge.

| # | Standard | Nivel curent | Nivel maxim (goal) | Distanță |
|---|----------|-------------|-------------------|----------|
| 1 | **OWASP ASVS** | L2 (100 req.) | **L3** (~130 req., verificare arhitecturală) | 🟡 Medie |
| 2 | **PCI DSS** | v4.0.1 (prin Stripe) | **v4.0.1** (toate cele 12 req.) | 🟢 Mică (Stripe le face) |
| 3 | **NIST CSF** | v2.0, Tier 2 | **v2.0 Tier 4** (Adaptive) | 🟡 Medie |
| 4 | **CIS Controls** | IG2 (~11/18 controale) | **IG3** (toate 18 controale) | 🟡 Medie |
| 5 | **GDPR** | Articolele 13, 17, 20 | **Toate articolele** (inclusiv 33-34 breach notification) | 🟡 Medie |
| 6 | **WCAG** | 2.1 Level AA | **2.2 Level AAA** | 🟡 Medie |
| 7 | **ISO 27001** | Documentat | **Certificat oficial** (audit extern) | 🔴 Mare |
| 8 | **eIDAS** | 1.0 (hash facturi) | **2.0** (EUDI Wallet, qualified signatures) | 🔴 Mare |
| 9 | **PSD2/SCA** | SCA prin Stripe | **PSD3** (2027+) | 🔴 Foarte mare |
| 10 | **HTTP/1.1** | RFC 7230-7235 | **HTTP/3** (RFC 9114, QUIC) | 🟡 Medie |
| 11 | **TLS** | 1.2+ | **TLS 1.3** (RFC 8446, zero RTT) | 🟢 Mică |
| 12 | **Rust** | Edition 2024, clippy clean | **Edition 2027** + `cargo audit` în CI | 🟡 Medie |
| 13 | **PostgreSQL** | 18 + pgvector | **19** + read replicas + PgBouncer | 🟡 Medie |
| 14 | **SOC 2** | Neimplementat | **Type II** (audit independent) | 🔴 Mare |
| 15 | **OWASP API Top 10** | 9/10 API-uri | **10/10** (inclusiv OpenAPI docs) | 🟢 Mică |
| 16 | **CIS Benchmarks** | Neimplementat | **Hardening Docker + Linux + PostgreSQL** | 🟡 Medie |

### Roadmap pe termen (2026–2030)

```
LEGENDĂ:
  ─── Implementare directă
  ··· Pregătire / cercetare
  ░░░ Dependent de mediu (Cloud / VPS / on-prem)

2026                    2027                    2028                    2029                    2030
├── ASVS L3 ───────────┤
├── HTTP/3 ────────────┤
├── TLS 1.3 ───────────┤
├── WCAG 2.2 AAA ──────┤
├── CIS IG3 ───────────┤
├── PgBouncer ─────────┤
├── Cargo audit CI ────┤
├── OpenAPI docs ──────┤
├── Secrets age init ──┤
├── Incident Response ─┤
│                        ├── ISO 27001 cert ─┤
│                        ├── Rust Ed. 2027 ─┤
│                        ├── SOC 2 Type I ──┤
│                        ├── eIDAS 2.0 ─────┤
│                        ├── PSD3 (2027+) ──┤
│                        ├── WASM prod ─────┤
│                        ├── WCAG 3.0 ──────┤
│                        ├── HTTP/2 mTLS ───┤
│                        ├── OCSP Stapling ─┤
│                        │                    ├── SOC 2 Type II ──┤
│                        │                    ├── FedRAMP ────────┤
│                        │                    ├── Quantum-safe ───┤
│                        │                    ├── Zero Trust ─────┤
│                        │                    ├── Confidential Computing ─┤
│                        │                    │                      ├── AI Security (OWASP LLM Top 10) ─┤
│                        │                    │                      ├── Post-quantum cryptography ─────┤
│                        │                    │                      ├── Autonomous compliance ─────────┤
│                        │                    │                      │
│  ░░░░░ Conexiune S22 ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
│  ░░░░░ Docker Bench ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
│  ░░░░░ Hardening OS ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
│  ░░░░░ Firewall ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
│  ░░░░░ mTLS ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
```

### Timeline detaliat

| Perioadă | Focus | Standarde vizate | Impact |
|----------|-------|------------------|--------|
| **2026 H2** | Securitate aplicație + infrastructură | ASVS L3, HTTP/3, TLS 1.3, WCAG 2.2 AAA, CIS IG3, PgBouncer | 🟡 Mediu — îmbunătățiri incrementale |
| **2027 H1** | Certificări + identitate digitală | ISO 27001 (audit intern), Rust Ed. 2027, eIDAS 2.0, PSD3 | 🔴 Mare — procese + legal |
| **2027 H2** | Platformă + scalare | SOC 2 Type I, WASM producție, HTTP/2 mTLS, OCSP Stapling | 🟡 Mediu — scalare |
| **2028 H1** | Conformitate avansată | SOC 2 Type II, Certificate Transparency, DNSSEC, mTLS | 🟡 Mediu |
| **2028 H2** | Cloud + guvernamental | FedRAMP, BSI C5, CIS Benchmarks (toate nivelele) | 🔴 Mare — entry US/EU |
| **2029** | Next-gen security | Quantum-safe cryptography, Zero Trust Architecture, Confidential Computing (TEE) | 🔴 Foarte mare — tehnologie nouă |
| **2030+** | AI + autonomie | OWASP LLM Top 10, AI security, Post-quantum crypto, Autonomous compliance | 🔴 Foarte mare — frontieră |

### Cost estimat per standard

| Standard | Efort implementare | Cost tooling | Timp |
|----------|-------------------|--------------|------|
| ASVS L3 | 2-3 săptămîni | 0 EUR (open source) | 2026 H2 |
| HTTP/3 (QUIC) | 1 zi (config) | 0 EUR (Axum/Tokio) | 2026 H2 |
| TLS 1.3 | 1 zi (config) | 0 EUR | 2026 H2 |
| WCAG 2.2 AAA | 2-3 săptămîni | 0 EUR | 2026 H2 |
| CIS IG3 | 1-2 luni | 0 EUR (open source) | 2026 H2–2027 H1 |
| PgBouncer | 1 săptămînă | 0 EUR | 2026 H2 |
| Cargo audit CI | 1 zi | 0 EUR | 2026 H2 |
| ISO 27001 | 6-12 luni | 5.000–15.000 EUR (auditor) | 2027 |
| SOC 2 Type I | 3-6 luni | 10.000–30.000 EUR (auditor) | 2027 |
| SOC 2 Type II | 6-12 luni | 20.000–50.000 EUR (auditor) | 2028 |
| eIDAS 2.0 | 2-3 luni | 0–5.000 EUR | 2027 |
| PSD3 | 3-6 luni | 0–10.000 EUR | 2027+ |
| FedRAMP | 12-24 luni | 100.000–500.000 EUR | 2028+ |
| Quantum-safe | 6-12 luni (cercetare) | 0 EUR (liboqs) | 2029 |
| Zero Trust | 3-6 luni | 0–20.000 EUR | 2028+ |

### Dependințe între standarde

```
ASVS L2 ───→ ASVS L3
    │
    ├──→ CIS IG2 ──→ CIS IG3 ──→ NIST CSF Tier 4
    │
    ├──→ PCI DSS ──→ SOC 2 Type I ──→ SOC 2 Type II ──→ FedRAMP
    │
    └──→ ISO 27001 ──→ ISO 27701 (privacy)
                            │
                            └──→ SOC 2 + GDPR + ePrivacy

WCAG 2.1 AA ──→ 2.2 AA ──→ 2.2 AAA ──→ 3.0
                    │
                    └──→ EN 301 549 (UE)
```

### Matrice RACI pe nivele

| Standard | Responsabil | Consultat | Informat |
|----------|-------------|-----------|----------|
| BIOS/UEFI hardening | DevOps | Security | Toți |
| Kernel + OS hardening | DevOps | Security | Toți |
| Docker hardening | DevOps | Security | Toți |
| ASVS / WCAG | Developer | Security, UX | Toți |
| GDPR / PCI DSS | Developer + Legal | Security | Toți |
| ISO 27001 / SOC 2 | Management | Legal, Security, DevOps | Toți |
| FedRAMP / BSI C5 | Management + Cloud | Legal, Security | Toți |

*Ultima actualizare: 2026-07-11*
*Mentenanță: se actualizează la fiecare implementare de standard nou*
