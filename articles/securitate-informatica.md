---
title: "Securitate informatica - principii, hash, si semnaturi"
slug: "securitate-informatica"
summary: "Securitatea informatică nu se face cu un singur algoritm, ci cu principii solide: confidentialitate, integritate, disponibilitate. Funcțiile hash, semnăturile digitale și infrastructura cu cheie publică (PKI) sunt instrumentele care implementează aceste principii."
category_path: ["Securitate și criptografie", "Principii"]
tags: ["securitate", "hash", "semnături", "confidențialitate", "integritate", "PKI", "criptografie"]
difficulty: "advanced"
reading_time: 18
related: ["criptografia-rsa", "entropia-informationala", "protocoale-tcp-ip", "dns-http-rutare", "p-vs-np"]
---

## Cele trei principii fundamentale (CIA)

| Principiu | Descriere | Încălcare |
|---|---|---|
| **Confidențialitate** | Datele sunt accesibile doar celor autorizați | Scurgere de date, spargere parole |
| **Integritate** | Datele nu sunt modificate neautorizat | Atac Man-in-the-Middle, modificare fișiere |
| **Disponibilitate** | Sistemul e funcțional când e nevoie | Atac DDoS, ransomware |

La acestea se adaugă:
- **Autenticitate** — știm cine a creat datele
- **Non-repudiere** — autorul nu poate nega că a creat datele
- **Autorizare** — fiecare utilizare are doar permisiunile necesare (principiul minimului privilegiu)

## Funcții hash criptografice

Un hash criptografic e o funcție care transformă date de orice dimensiune într-un rezumat de dimensiune fixă (de obicei 256 sau 512 biți).

### Proprietăți esențiale

1. **Unidirecțională** — din hash nu poți recupera datele originale
2. **Rezistentă la coliziuni** — e computațional imposibil să găsești două intrări cu același hash
3. **Avalanșă** — o modificare de 1 bit în intrare schimbă ~50% din biții hash-ului

### Algoritmi

| Algoritm | Dimensiune hash | Status |
|---|---|---|
| MD5 | 128 biți | **Nesigur** — coliziuni găsite în 2004 |
| SHA-1 | 160 biți | **Nesigur** — coliziuni practice în 2017 |
| SHA-256 | 256 biți | Sigur (folosit în Bitcoin, TLS) |
| SHA-3 | 256/512 biți | Sigur (standardul cel mai recent) |
| BLAKE3 | 256 biți | Sigur, foarte rapid |

### Aplicații

- **Stocarea parolelor:** Se hash-uiește parola (cu salt!) și se stochează doar hash-ul
- **Verificarea integrității:** Hash-uiești un fișier la download și compari cu hash-ul oficial
- **Blockchain:** Lanțul de blocuri e legat prin hash-uri
- **Git:** Fiecare commit e identificat prin hash-ul său SHA-1

## Semnături digitale

O semnătură digitală demonstrează că un document a fost creat de o anumită persoană și nu a fost modificat.

### Cum funcționează

1. Expeditorul hash-uiește documentul: h = H(doc)
2. Expeditorul criptează hash-ul cu cheia sa privată: s = E(h, K_privat)
3. Expeditorul trimite documentul + semnătura (doc, s)
4. Destinatarul decriptează semnătura cu cheia publică a expeditorului: h = D(s, K_public)
5. Destinatarul hash-uiește documentul și compară cu h

### Proprietăți

- **Autenticitate:** Doar deținătorul cheii private poate crea semnătura
- **Integritate:** Orice modificare a documentului invalidează semnătura
- **Non-repudiere:** Expeditorul nu poate nega că a semnat

### PKI — Infrastructura cu cheie publică

PKI rezolvă problema: „cum știu că această cheie publică chiar aparține cui spune că aparține?"

**Autoritatea de Certificare (CA):** O terță parte de încredere care emite certificate digitale. Un certificat digital leagă o cheie publică de o identitate (nume, domeniu, organizație).

**Lanțul de încredere:**
```
CA Rădăcină (self-signed)
  ├── CA Intermediară 1
  │   ├── example.com (certificat)
  │   └── google.com (certificat)
  └── CA Intermediară 2
      └── ...
```

Browserul tău are încredere în CA Rădăcină. Pe baza asta, are încredere în toate certificatele semnate de CA-urile din lanț.

## Principiul lui Kerckhoffs

> Un sistem criptografic trebuie să fie sigur chiar dacă totul despre el — cu excepția cheii — e cunoscut public.

Cu alte cuvinte: **securitatea prin obscuritate nu e securitate**. Algoritmii trebuie să fie publici și analizați de comunitate. Cheia e singurul secret.

## Atacuri comune

| Atac | Ținta | Contramăsura |
|---|---|---|
| Man-in-the-Middle | Confidențialitate | TLS, certificate |
| DDoS | Disponibilitate | Load balancing, rate limiting |
| SQL Injection | Integritate | Prepared statements |
| Cross-Site Scripting (XSS) | Confidențialitate | Input sanitization |
| Phishing | Autenticitate | Multi-factor authentication |
| Rainbow table | Confidențialitate (parole) | Salt + hash lent (bcrypt, argon2) |

