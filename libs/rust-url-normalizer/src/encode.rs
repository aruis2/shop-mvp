// =============================================================================
// 🔗 URL encoding simplu — 0 dependințe externe
// =============================================================================
// Ne trebuie doar pentru a pune mesaje de eroare în query params:
//   /checkout?error=Ceva+a+n+mers+gresit
// Fără encoding, spațiile și caracterele speciale sparg URL-ul.
// =============================================================================

/// Encodează un string pentru a fi folosit în query params (application/x-www-form-urlencoded).
/// Spațiul devine `+`, caracterele speciale devin `%XX`.
pub fn url_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => {
                result.push('+');
            }
            _ => {
                result.push('%');
                result.push_str(&hex_encode(byte));
            }
        }
    }
    result
}

fn hex_encode(byte: u8) -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut s = String::with_capacity(2);
    s.push(HEX[(byte >> 4) as usize] as char);
    s.push(HEX[(byte & 0x0F) as usize] as char);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode_simple() {
        assert_eq!(url_encode("hello"), "hello");
        assert_eq!(url_encode("hello world"), "hello+world");
        assert_eq!(url_encode("a+b"), "a%2Bb");
        assert_eq!(url_encode("100% garanție"), "100%25+garan%C8%9Bie");
    }

    #[test]
    fn test_url_encode_special_chars() {
        assert_eq!(url_encode("&<>\"'"), "%26%3C%3E%22%27");
        assert_eq!(url_encode("?error=test"), "%3Ferror%3Dtest");
    }

    #[test]
    fn test_url_encode_already_encoded() {
        assert_eq!(url_encode("%20"), "%2520");
    }
}
