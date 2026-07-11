// =============================================================================
// 🛤️ rust-path-prefix — URL path prefix management for reverse proxy setups
// =============================================================================
//
// Utilities for apps deployed behind a path-prefixed reverse proxy.
// Reads BASE_PATH env var and provides helpers to prefix/strip URL paths.
//
// Usage:
//   let pfx = PathPrefix::from_env();
//   let url = pfx.prepend("/products");  // → "/shop/products"
//   let path = pfx.strip("/shop/products");  // → "/products"
// =============================================================================

use std::env;

/// Manages URL path prefixing for apps behind a reverse proxy.
///
/// Reads `BASE_PATH` from environment (defaults to empty string).
/// When empty, all methods are no-ops (pass-through).
#[derive(Clone, Debug)]
pub struct PathPrefix {
    prefix: String,
}

impl PathPrefix {
    /// Creates a new `PathPrefix` from the `BASE_PATH` environment variable.
    /// Defaults to `""` (no prefix) if not set.
    pub fn from_env() -> Self {
        let prefix = env::var("BASE_PATH").unwrap_or_default();
        Self::new(&prefix)
    }

    /// Creates a new `PathPrefix` with the given prefix.
    /// Trailing slash is stripped from the prefix.
    pub fn new(prefix: &str) -> Self {
        let p = prefix.trim_end_matches('/').to_string();
        Self { prefix: p }
    }

    /// Returns the base path prefix (e.g. `"/shop"`).
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Prepends the base path to a URL path.
    ///
    /// If `BASE_PATH` is `/shop`, then:
    /// - `prepend("/products")` → `"/shop/products"`
    /// - `prepend("/")` → `"/shop/"`
    /// - `prepend("")` → `"/shop"`
    ///
    /// If `BASE_PATH` is empty, returns the input unchanged.
    pub fn prepend(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            return path.to_string();
        }
        if path == "/" || path.is_empty() {
            format!("{}/", self.prefix)
        } else {
            format!("{}{}", self.prefix, path)
        }
    }

    /// Strips the base path prefix from a request path.
    ///
    /// If `BASE_PATH` is `/shop`, then:
    /// - `strip("/shop/products")` → `"/products"`
    /// - `strip("/shop/")` → `"/"`
    /// - `strip("/shop")` → `"/"`
    /// - `strip("/other")` → `"/other"` (no match)
    ///
    /// If `BASE_PATH` is empty, returns the input unchanged.
    pub fn strip(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            return path.to_string();
        }
        if let Some(rest) = path.strip_prefix(&self.prefix) {
            if rest.is_empty() || rest == "/" {
                "/".to_string()
            } else {
                rest.to_string()
            }
        } else {
            path.to_string()
        }
    }

    /// Strips the base path from an Axum-compatible path (with `{*rest}` suffix).
    /// The `prefix` parameter is the route prefix (e.g. "/shop").
    pub fn strip_prefix(&self, path: &str, route_prefix: &str) -> String {
        let p = route_prefix.trim_end_matches('*').trim_end_matches('/');
        if let Some(rest) = path.strip_prefix(p) {
            if rest.is_empty() || rest == "/" {
                "/".to_string()
            } else {
                rest.to_string()
            }
        } else {
            path.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_prefix() {
        let p = PathPrefix::new("");
        assert_eq!(p.prepend("/products"), "/products");
        assert_eq!(p.strip("/products"), "/products");
        assert_eq!(p.prefix(), "");
    }

    #[test]
    fn test_prepend() {
        let p = PathPrefix::new("/shop");
        assert_eq!(p.prepend("/products"), "/shop/products");
        assert_eq!(p.prepend("/"), "/shop/");
        assert_eq!(p.prepend(""), "/shop");
        assert_eq!(p.prepend("/cart/add"), "/shop/cart/add");
    }

    #[test]
    fn test_strip() {
        let p = PathPrefix::new("/shop");
        assert_eq!(p.strip("/shop/products"), "/products");
        assert_eq!(p.strip("/shop/"), "/");
        assert_eq!(p.strip("/shop"), "/");
        assert_eq!(p.strip("/other"), "/other");
    }

    #[test]
    fn test_trailing_slash_normalization() {
        let p = PathPrefix::new("/shop/");  // trailing slash stripped
        assert_eq!(p.prefix(), "/shop");
        assert_eq!(p.prepend("/products"), "/shop/products");
    }
}
