// =============================================================================
// 🚦 Rate limiter simplu, in-memory (fără dependințe externe)
// =============================================================================
// Folosire:
//   let limiter = RateLimiter::new(5, 60);  // 5 requesturi/minute
//   if !limiter.check("192.168.1.1") { return "Too Many Requests"; }
//
// HN-style: simplu, predictibil, fără Redis sau cache extern.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

pub struct RateLimiter {
    max_requests: usize,
    window_secs: u64,
    requests: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            requests: Mutex::new(HashMap::new()),
        }
    }

    /// Verifică dacă un IP poate face request.
    /// Returnează `true` dacă e permis, `false` dacă e rate-limited.
    pub fn check(&self, ip: &str) -> bool {
        let now = Instant::now();
        let mut map = self.requests.lock().expect("ratelimit Mutex poisoned");
        let entries = map.entry(ip.to_string()).or_insert_with(Vec::new);

        // Elimină entry-urile mai vechi de fereastră
        let cutoff = now - std::time::Duration::from_secs(self.window_secs);
        entries.retain(|&t| t > cutoff);

        if entries.len() >= self.max_requests {
            tracing::warn!(target: "ratelimit", "Rate limit depășit pentru {}. {} requesturi în {}s", ip, entries.len(), self.window_secs);
            false
        } else {
            entries.push(now);
            true
        }
    }

    /// Curăță IP-urile expirate (opțional, pentru economie de memorie)
    pub fn cleanup(&self) {
        let now = Instant::now();
        let cutoff = now - std::time::Duration::from_secs(self.window_secs);
        let mut map = self.requests.lock().expect("ratelimit Mutex poisoned");
        map.retain(|_, entries| {
            entries.retain(|&t| t > cutoff);
            !entries.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_rate_limit() {
        let limiter = RateLimiter::new(2, 60);
        assert!(limiter.check("1.2.3.4"));
        assert!(limiter.check("1.2.3.4"));
        assert!(!limiter.check("1.2.3.4")); // al 3-lea e refuzat
        assert!(limiter.check("5.6.7.8")); // alt IP, permis
    }
}
