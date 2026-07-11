---
title: "PSD2 / SCA — Strong Customer Authentication pentru plăți online"
description: "Implementarea cerințelor PSD2 și SCA în shop-mvp, conform reglementărilor europene pentru plăți electronice"
date: 2026-07-11
---

# PSD2 / SCA — Strong Customer Authentication

## Cuprins

1. [Ce este PSD2](#ce-este-psd2)
2. [Ce este SCA](#ce-este-sca)
3. [Cerințe tehnice](#cerințe-tehnice)
4. [Implementare în shop-mvp](#implementare-în-shop-mvp)
5. [Verificare și testare](#verificare-și-testare)

## Ce este PSD2

**PSD2** (Payment Services Directive 2) este Directiva Europeană 2015/2366, care reglementează serviciile de plată în Uniunea Europeană. A intrat în vigoare în ianuarie 2018, iar cerințele SCA (Strong Customer Authentication) au devenit obligatorii din 14 septembrie 2019.

### Obiective principale

1. **Creșterea securității plăților** — prin autentificarea puternică a clientului (SCA)
2. **Inovație și competiție** — prin deschiderea pieței către terți (TPP)
3. **Protecția consumatorului** — prin reguli clare de răspundere și rambursare

## Ce este SCA

**SCA** (Strong Customer Authentication) cere ca plățile electronice să fie autentificate folosind **cel puțin două** din următoarele trei categorii:

| Factor | Exemple |
|--------|---------|
| **Ce știi** | Parolă, PIN, întrebare secretă |
| **Ce ai** | Telefon, card fizic, token hardware |
| **Ce ești** | Amprentă, față, iris |

### Excepții de la SCA

- Plăți sub 30 EUR (cu limite cumulative)
- Plăți recurente (abonamente) la aceeași valoare
- Transferuri între conturi proprii
- Plăți către comercianți de încredere (whitelist)
- Plăți contactless sub 50 EUR

## Implementare în shop-mvp

Shop-mvp folosește **Stripe Checkout** care gestionează automat SCA prin:

1. **Payment Intents API** — Stripe creează un `PaymentIntent` care știe dacă SCA e necesar
2. **3D Secure** — Stripe declanșează 3D Secure automat când banca emitentă îl cere
3. **Webhook-uri** — Stripe notifică statusul plății (inclusiv fallback SCA)

### Fluxul plății cu SCA

```
Client → Checkout → Stripe Checkout Session
                       ↓
                 3D Secure (dacă e necesar)
                       ↓
              Stripe Webhook → shop-mvp
                       ↓
              Comandă confirmată
```

### Cod — Stripe Payment cu Retry

```rust
// shop-mvp/src/payment_retry.rs
// RetryPayment adaugă retry cu exponential backoff + timeout
// SCA este gestionat de Stripe prin Payment Intents API

async fn create_checkout(&self, req: CreateCheckoutRequest) -> Result<CheckoutResponse, PaymentError> {
    let mut last_err = None;
    for attempt in 0..=self.max_retries {
        if attempt > 0 {
            let delay = self.base_delay_ms * (1u64 << (attempt - 1));
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
        match timeout(self.request_timeout_ms, self.inner.create_checkout(cloned)).await {
            Ok(Ok(resp)) => return Ok(resp),
            Ok(Err(e)) => { /* retry only on transient */ }
            Err(_) => { /* timeout */ }
        }
    }
    Err(last_err.unwrap())
}
```

### Verificare status plată

```rust
// Verificare Stripe payment status
pub async fn verify_payment(payment_provider_id: &str) -> Result<PaymentStatus, PaymentError> {
    // Stripe Checkout Session.Status → "complete" | "expired" | "open"
    // SCA completează automat prin 3D Secure
}
```

## Verificare și testare

### Testare SCA cu carduri Stripe

| Card | Comportament SCA |
|------|------------------|
| `4242 4242 4242 4242` | Autentificare reușită (SCA bypass) |
| `4000 0025 0000 3155` | Necesită 3D Secure (SCA activ) |
| `4000 0027 6000 3184` | 3D Secure eșuează |

### Comenzi curl

```bash
# Simulează checkout cu card SCA
curl -X POST http://localhost:3001/checkout \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "session_id=test&guest_email=test@example.com&shipping_name=Test&shipping_address=Str. Test&shipping_phone=0712345678"
```

### Verificare header-e SCA

```bash
curl -s -D- http://localhost:3001/checkout | grep -i "strict-transport-security"
# strict-transport-security: max-age=31536000; includeSubDomains
```

## Referințe

- [Directiva PSD2 (EUR-Lex)](https://eur-lex.europa.eu/eli/dir/2015/2366)
- [Stripe SCA Guide](https://stripe.com/docs/strong-customer-authentication)
- [EBA Regulatory Technical Standards](https://www.eba.europa.eu/regulation-and-policy/consumer-protection-and-financial-innovation)
