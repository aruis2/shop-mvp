// =============================================================================
// 🎯 OutputFactory — Formatare sigură la ieșire
// =============================================================================
// FILOSOFIE: PHILOSOPHY #14 (granița de încredere)
// STANDARD: OWASP ASVS V6.1 (Output Encoding)
// =============================================================================
//
// OutputFactory e SIMETRIC cu InputFactory:
//   InputFactory:  granița de INTRARE  (browser → Rust)
//   OutputFactory: granița de IEȘIRE   (Rust → Tera → browser)
//
// DE CE: Nu avem încredere deplină în Tera (template engine).
//        Dacă Tera are un bug la auto-escape, OutputFactory încă protejează.
//        Datele sînt filtrate de ULTIMUL nostru cod înainte să părăsească
//        zona de încredere (Rust).
//
// =============================================================================

use crate::types::email::Email;
use crate::types::price::Price;
use crate::types::phone::PhoneNumber;
use crate::types::quantity::Quantity;
use crate::types::slug::Slug;
use crate::types::text::*;
use crate::types::currency::Currency;

/// OutputFactory — formatare sigură pentru exterior.
/// Toate datele care ies din Rust spre Tera trec prin aici.
pub struct OutputFactory;

impl OutputFactory {
    // ─── Email ────────────────────────────────────────────
    pub fn email_html(email: &Email) -> String {
        // Email e garantat valid de InputFactory, dar îl scăpăm oricum
        html_encode(email.as_str())
    }

    // ─── Preț ─────────────────────────────────────────────
    pub fn price_lei(price: &Price) -> String {
        price.as_lei_str() // "249.99" — garantat formatat corect
    }

    // ─── Telefon ──────────────────────────────────────────
    pub fn phone_display(phone: &PhoneNumber) -> String {
        phone.to_string() // "0712 345 678"
    }

    // ─── Text (nume, adresă, note, brand, produs) ─────────
    pub fn text_html(s: &str) -> String {
        html_encode(s)
    }

    // ─── Slug ─────────────────────────────────────────────
    pub fn slug_url(slug: &Slug) -> String {
        slug.as_str().to_string() // garantat URL-safe de InputFactory
    }

    // ─── Cantitate ────────────────────────────────────────
    pub fn quantity_display(qty: &Quantity) -> String {
        qty.get().to_string()
    }

    // ─── Valută ───────────────────────────────────────────
    pub fn currency_display(currency: &Currency) -> String {
        currency.to_string() // "RON", "USD", "EUR"
    }
}

/// HTML encode — înlocuiește caracterele speciale cu entități HTML.
/// Asta e REDUNDANT cu Tera auto-escape, dar:
/// 1. Dacă Tera are un bug, noi încă protejăm
/// 2. Dacă datele merg în JSON/CSV/email, tot sînt sigure
/// 3. E ultimul nostru cod înainte ca datele să părăsească Rust
fn html_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_encode_basic() {
        assert_eq!(html_encode("hello"), "hello");
    }

    #[test]
    fn html_encode_escapes() {
        assert_eq!(html_encode("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn html_encode_quotes() {
        assert_eq!(html_encode("\"test\""), "&quot;test&quot;");
    }

    #[test]
    fn html_encode_amp() {
        assert_eq!(html_encode("a&b"), "a&amp;b");
    }

    #[test]
    fn price_output() {
        let p = Price::new(24999).unwrap();
        assert_eq!(OutputFactory::price_lei(&p), "249.99");
    }

    #[test]
    fn email_output() {
        let e = Email::parse("test@test.com").unwrap();
        assert_eq!(OutputFactory::email_html(&e), "test@test.com");
    }

    #[test]
    fn email_xss_in_name() {
        // Dacă DB e coruptă și conține <script> în email
        // Email::parse() l-ar respinge, dar dacă a intrat direct în DB...
        // OutputFactory îl face inofensiv
        let malicious = "<script>alert(1)</script>";
        assert_eq!(html_encode(malicious),
                   "&lt;script&gt;alert(1)&lt;/script&gt;");
    }
}
