/// Generează un slug URL-friendly dintr-un text
///
/// # Exemplu
/// ```rust
/// use rust_slug::generate_slug;
///
/// assert_eq!(generate_slug("Căști Sony WH-1000XM5"), "casti-sony-wh-1000xm5");
/// assert_eq!(generate_slug("Laptop Dell XPS 13"), "laptop-dell-xps-13");
/// ```
pub fn generate_slug(text: &str) -> String {
    text
        .to_lowercase()
        .replace('ă', "a")
        .replace('â', "a")
        .replace('î', "i")
        .replace('ș', "s")
        .replace('ț', "t")
        .replace(' ', "-")
        .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "")
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Slug pentru listing (fără UUID)
pub fn generate_listing_slug(title: &str) -> String {
    generate_slug(title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_slug() {
        assert_eq!(generate_slug("Laptop Dell XPS 13"), "laptop-dell-xps-13");
        assert_eq!(generate_slug("Telefon iPhone 16 Pro"), "telefon-iphone-16-pro");
        assert_eq!(generate_slug("Căști Sony WH-1000XM5"), "casti-sony-wh-1000xm5");
    }

    #[test]
    fn test_generate_listing_slug() {
        assert_eq!(
            generate_listing_slug("Laptop Dell XPS 13"),
            "laptop-dell-xps-13"
        );
    }

    #[test]
    fn test_slug_special_chars() {
        assert_eq!(generate_slug("Telefon & laptop"), "telefon-laptop");
        assert_eq!(generate_slug("Preț: 100 lei"), "pret-100-lei");
    }

    #[test]
    fn test_slug_empty() {
        assert_eq!(generate_slug(""), "");
    }

    #[test]
    fn test_slug_multiple_spaces() {
        assert_eq!(generate_slug("Laptop   Dell"), "laptop-dell");
    }
}