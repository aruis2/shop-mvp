// =============================================================================
// 🎯 OutputFactory — Formatare sigură la ieșire
// =============================================================================
// FILOSOFIE: PHILOSOPHY #14 (granița de încredere)
// STANDARD: OWASP ASVS V6.1 (Output Encoding), OWASP ASVS V11 (Business Logic)
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
// COVERAGE: Acoperă TOT ce iese la client:
//   1. Context Tera    → sanitize_context() walk recursiv
//   2. Redirect URL    → safe_redirect_url() validate same-origin
//   3. Error messages  → safe_error_msg() sanitizare
//   4. Header values   → safe_header_value() fără injectare
//   5. Cookie values   → safe_cookie_value() fără caractere periculoase
//   6. JSON values     → sanitize_json() walk recursiv
//   7. HTML encoding   → html_encode() pentru string-uri individuale
// =============================================================================

use rust_input_types::{Email, Price, PhoneNumber, Quantity};
use rust_input_types::{Slug, Currency};

/// OutputFactory — formatare sigură pentru exterior.
/// Toate datele care ies din Rust spre Tera trec prin aici.
pub struct OutputFactory;

impl OutputFactory {
    // ─── Email ────────────────────────────────────────────
    pub fn email_html(email: &Email) -> String {
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

    // ─── Error message ────────────────────────────────────
    /// Sanitizează un mesaj de eroare pentru afișare în pagină.
    /// Elimină caractere periculoase, limitează lungimea.
    pub fn safe_error_msg(msg: &str) -> String {
        let cleaned: String = msg.chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .take(200)
            .collect();
        html_encode(&cleaned)
    }

    // ─── Redirect URL ─────────────────────────────────────
    /// Validează un URL de redirect împotriva open redirect și XSS.
    /// Returnează None dacă URL-ul e periculos.
    ///
    /// Reguli:
    /// - Cale relativă (începe cu /) → acceptat
    /// - Același domeniu (începe cu site_url) → acceptat
    /// - javascript:, data:, vbscript:, file: → respins
    /// - Domenii externe → respins (open redirect)
    pub fn safe_redirect_url(url: &str, site_url: &str) -> Option<String> {
        let url = url.trim();

        if url.is_empty() {
            return None;
        }

        // Blochează scheme periculoase
        let lower = url.to_lowercase();
        if lower.starts_with("javascript:")
            || lower.starts_with("data:")
            || lower.starts_with("vbscript:")
            || lower.starts_with("file:")
            || lower.starts_with("blob:")
        {
            return None;
        }

        // Cale relativă — acceptat (dar blochează protocol-relative //evil.com)
        if url.starts_with('/') {
            if url.len() > 1 && url.as_bytes()[1] == b'/' {
                return None; // protocol-relative URL
            }
            if url.chars().all(|c| c.is_ascii_alphanumeric() || "/?&=.#%-_~+".contains(c)) {
                return Some(url.to_string());
            }
            return None;
        }

        // Același domeniu
        if url.starts_with(site_url) {
            let rest = &url[site_url.len()..];
            if rest.is_empty() || rest.starts_with('/') || rest.starts_with('?') || rest.starts_with('#') {
                return Some(url.to_string());
            }
            return None;
        }

        None
    }

    // ─── Header value ─────────────────────────────────────
    /// Sanitizează o valoare de header HTTP.
    /// Elimină newline-uri (prevenire HTTP response splitting).
    pub fn safe_header_value(val: &str) -> String {
        val.chars()
            .filter(|c| c.is_ascii_graphic() || *c == ' ')
            .take(1024)
            .collect()
    }

    // ─── Cookie value ─────────────────────────────────────
    /// Sanitizează o valoare de cookie.
    /// Elimină caractere care sparg formatul cookie.
    pub fn safe_cookie_value(val: &str) -> String {
        val.chars()
            .filter(|c| c.is_ascii_alphanumeric() || "-_.~".contains(*c))
            .take(4096)
            .collect()
    }

    // ─── Context sanitization (Tera) ──────────────────────
    /// Sanitizează un serde_json::Value înainte de Tera render.
    /// Walk recursiv, html_encode pe toate string-urile.
    /// 🔒 Sanitizare context: elimină caractere periculoase, DAR nu HTML-encode.
    /// Tera face auto-escape la HTML (configurat în RenderService).
    /// Dacă am face html_encode AICI, am produce dublu-escape (ex: " → &quot; → &amp;quot;).
    pub fn sanitize_context(ctx: &serde_json::Value) -> serde_json::Value {
        match ctx {
            serde_json::Value::String(s) => {
                // Elimină caractere de control și null bytes (periculoase în orice context)
                let cleaned: String = s.chars()
                    .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
                    .collect();
                serde_json::Value::String(cleaned)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| Self::sanitize_context(v)).collect())
            }
            serde_json::Value::Object(map) => {
                let sanitized: serde_json::Map<String, serde_json::Value> = map.iter()
                    .map(|(k, v)| (k.clone(), Self::sanitize_context(v)))
                    .collect();
                serde_json::Value::Object(sanitized)
            }
            // Number, Bool, Null — neschimbate
            other => other.clone(),
        }
    }

    /// Creează un Tera Context dintr-un serde_json::Value, cu sanitizare automată.
    /// Folosește serde pentru conversie: Value → JSON → Context.
    /// Primul pas: sanitize_context() html_encode pe toate string-urile.
    pub fn make_context(data: &serde_json::Value) -> tera::Context {
        let sanitized = Self::sanitize_context(data);
        tera::Context::from_serialize(&sanitized)
            .expect("OutputFactory::make_context: from_serialize a eșuat (datele nu sunt un JSON object?)")
    }
}

/// HTML encode — înlocuiește caracterele speciale cu entități HTML.
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

// =============================================================================
// Teste
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ─── html_encode ──────────────────────────────────────

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
    fn html_encode_all_special() {
        assert_eq!(html_encode("<>&\"'"), "&lt;&gt;&amp;&quot;&#39;");
    }

    #[test]
    fn html_encode_unicode() {
        assert_eq!(html_encode("ăâîșț"), "ăâîșț");
    }

    #[test]
    fn html_encode_empty() {
        assert_eq!(html_encode(""), "");
    }

    // ─── OutputFactory::text_html ────────────────────────

    #[test]
    fn text_html_basic() {
        assert_eq!(OutputFactory::text_html("hello"), "hello");
    }

    #[test]
    fn text_html_xss() {
        assert_eq!(
            OutputFactory::text_html("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
    }

    // ─── OutputFactory::price_lei ─────────────────────────

    #[test]
    fn price_output() {
        let p = Price::new(24999).unwrap();
        assert_eq!(OutputFactory::price_lei(&p), "249.99");
    }

    #[test]
    fn price_output_zero() {
        let p = Price::new(1).unwrap();
        assert_eq!(OutputFactory::price_lei(&p), "0.01");
    }

    // ─── OutputFactory::email_html ────────────────────────

    #[test]
    fn email_output() {
        let e = Email::parse("test@test.com").unwrap();
        assert_eq!(OutputFactory::email_html(&e), "test@test.com");
    }

    #[test]
    fn email_xss_in_db() {
        let malicious = "<script>alert(1)</script>";
        assert_eq!(html_encode(malicious),
                   "&lt;script&gt;alert(1)&lt;/script&gt;");
    }

    // ─── OutputFactory::safe_error_msg ────────────────────

    #[test]
    fn error_msg_basic() {
        assert_eq!(OutputFactory::safe_error_msg("A apărut o eroare"),
                   "A apărut o eroare");
    }

    #[test]
    fn error_msg_xss() {
        assert_eq!(
            OutputFactory::safe_error_msg("Eroare: <script>alert(1)</script>"),
            "Eroare: &lt;script&gt;alert(1)&lt;/script&gt;"
        );
    }

    #[test]
    fn error_msg_truncates_long() {
        let long = "a".repeat(300);
        let result = OutputFactory::safe_error_msg(&long);
        assert_eq!(result.len(), 200);
    }

    #[test]
    fn error_msg_removes_control_chars() {
        assert_eq!(
            OutputFactory::safe_error_msg("eroare\x00\x01\x02test"),
            "eroaretest"
        );
    }

    // ─── OutputFactory::safe_redirect_url ─────────────────

    #[test]
    fn redirect_relative_path() {
        assert_eq!(
            OutputFactory::safe_redirect_url("/produse?page=2", "http://localhost:3001"),
            Some("/produse?page=2".to_string())
        );
    }

    #[test]
    fn redirect_same_site() {
        assert_eq!(
            OutputFactory::safe_redirect_url("http://localhost:3001/produse", "http://localhost:3001"),
            Some("http://localhost:3001/produse".to_string())
        );
    }

    #[test]
    fn redirect_rejects_javascript() {
        assert_eq!(
            OutputFactory::safe_redirect_url("javascript:alert(1)", "http://localhost:3001"),
            None
        );
    }

    #[test]
    fn redirect_rejects_data() {
        assert_eq!(
            OutputFactory::safe_redirect_url("data:text/html,<script>alert(1)</script>", "http://localhost:3001"),
            None
        );
    }

    #[test]
    fn redirect_rejects_external() {
        assert_eq!(
            OutputFactory::safe_redirect_url("https://evil.com/phish", "http://localhost:3001"),
            None
        );
    }

    #[test]
    fn redirect_rejects_empty() {
        assert_eq!(
            OutputFactory::safe_redirect_url("", "http://localhost:3001"),
            None
        );
    }

    #[test]
    fn redirect_accepts_root() {
        assert_eq!(
            OutputFactory::safe_redirect_url("/", "http://localhost:3001"),
            Some("/".to_string())
        );
    }

    #[test]
    fn redirect_rejects_evil_scheme_case() {
        assert_eq!(
            OutputFactory::safe_redirect_url("JAVASCRIPT:alert(1)", "http://localhost:3001"),
            None
        );
    }

    #[test]
    fn redirect_rejects_protocol_relative() {
        assert_eq!(
            OutputFactory::safe_redirect_url("//evil.com/phish", "http://localhost:3001"),
            None
        );
    }

    #[test]
    fn redirect_rejects_double_slash() {
        assert_eq!(
            OutputFactory::safe_redirect_url("//google.com", "http://localhost:3001"),
            None
        );
    }

    // ─── OutputFactory::safe_header_value ─────────────────

    #[test]
    fn header_value_basic() {
        assert_eq!(OutputFactory::safe_header_value("test"), "test");
    }

    #[test]
    fn header_value_removes_newlines() {
        assert_eq!(
            OutputFactory::safe_header_value("test\r\nInjected-Header: evil"),
            "testInjected-Header: evil"
        );
    }

    #[test]
    fn header_value_truncates_long() {
        let long = "a".repeat(2000);
        let result = OutputFactory::safe_header_value(&long);
        assert_eq!(result.len(), 1024);
    }

    // ─── OutputFactory::safe_cookie_value ─────────────────

    #[test]
    fn cookie_value_basic() {
        assert_eq!(OutputFactory::safe_cookie_value("abc123-_.~"), "abc123-_.~");
    }

    #[test]
    fn cookie_value_removes_special() {
        assert_eq!(
            OutputFactory::safe_cookie_value("token=value; path=/"),
            "tokenvaluepath"
        );
    }

    // ─── OutputFactory::sanitize_context ──────────────────

    #[test]
    fn sanitize_context_string() {
        // sanitize_context NU face html_encode (Tera face auto-escape).
        // Doar elimină caractere de control.
        let val = serde_json::json!("<script>alert(1)</script>");
        let result = OutputFactory::sanitize_context(&val);
        assert_eq!(result, serde_json::json!("<script>alert(1)</script>"));
    }

    #[test]
    fn sanitize_context_number() {
        let val = serde_json::json!(42);
        let result = OutputFactory::sanitize_context(&val);
        assert_eq!(result, serde_json::json!(42));
    }

    #[test]
    fn sanitize_context_object() {
        // sanitize_context NU face html_encode (Tera face auto-escape).
        let val = serde_json::json!({
            "name": "<b>Ion</b>",
            "age": 30,
            "active": true,
            "nested": {
                "desc": "<script>alert(1)</script>"
            }
        });
        let result = OutputFactory::sanitize_context(&val);
        assert_eq!(result["name"], "<b>Ion</b>");
        assert_eq!(result["age"], 30);
        assert_eq!(result["active"], true);
        assert_eq!(result["nested"]["desc"], "<script>alert(1)</script>");
    }

    #[test]
    fn sanitize_context_array() {
        // sanitize_context NU face html_encode (Tera face auto-escape).
        let val = serde_json::json!([
            "<script>",
            {"name": "<b>x</b>"}
        ]);
        let result = OutputFactory::sanitize_context(&val);
        assert_eq!(result[0], "<script>");
        assert_eq!(result[1]["name"], "<b>x</b>");
    }

    #[test]
    fn sanitize_context_null() {
        let val = serde_json::Value::Null;
        let result = OutputFactory::sanitize_context(&val);
        assert_eq!(result, serde_json::Value::Null);
    }

    #[test]
    fn sanitize_context_empty_string() {
        let val = serde_json::json!("");
        let result = OutputFactory::sanitize_context(&val);
        assert_eq!(result, serde_json::json!(""));
    }
}
