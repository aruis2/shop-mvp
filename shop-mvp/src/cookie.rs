// =============================================================================
// 🍪 Cookie helper — fără dependințe externe
// =============================================================================

/// Citește un cookie după nume din header-ul `Cookie`
pub fn get_cookie<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix(&format!("{}=", name)) {
            return Some(val);
        }
    }
    None
}

/// Creează un header `Set-Cookie`
pub fn set_cookie(name: &str, value: &str, max_age_secs: i64) -> String {
    format!(
        "{name}={value}; HttpOnly; Path=/; SameSite=Lax; Max-Age={max_age_secs}"
    )
}

/// Șterge un cookie (setează Max-Age=0)
pub fn remove_cookie(name: &str) -> String {
    format!("{name}=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0")
}
