use std::collections::HashMap;
use std::sync::RwLock;

pub struct SlidingWindowRateLimiter {
    buckets: RwLock<HashMap<String, Vec<std::time::Instant>>>,
}

impl Default for SlidingWindowRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl SlidingWindowRateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
        }
    }

    pub fn check(&self, key: &str, max_count: usize, window_secs: u64) -> bool {
        let now = std::time::Instant::now();
        let window = std::time::Duration::from_secs(window_secs);
        let mut buckets = self.buckets.write().expect("rate limiter lock");

        let timestamps = buckets.entry(key.to_string()).or_default();

        timestamps.retain(|&t| now.duration_since(t) < window);

        if timestamps.len() >= max_count {
            return false;
        }

        timestamps.push(now);
        true
    }
}

/// Global rate limiter shared across both runtimes (TCP + Actix).
pub static RATE_LIMITER: std::sync::OnceLock<SlidingWindowRateLimiter> = std::sync::OnceLock::new();

pub fn global_limiter() -> &'static SlidingWindowRateLimiter {
    RATE_LIMITER.get_or_init(SlidingWindowRateLimiter::new)
}

// ---------------------------------------------------------------------------
// Rate limit presets
// ---------------------------------------------------------------------------

/// Per-IP search rate: 60 requests per minute
pub const SEARCH_RATE_MAX: usize = 60;
pub const SEARCH_RATE_WINDOW_SECS: u64 = 60;

/// Per-token create listing rate: 10 creations per minute
pub const CREATE_LISTING_RATE_MAX: usize = 10;
pub const CREATE_LISTING_RATE_WINDOW_SECS: u64 = 60;

/// Per-token open negotiation rate: 20 per minute
pub const OPEN_NEGOTIATION_RATE_MAX: usize = 20;
pub const OPEN_NEGOTIATION_RATE_WINDOW_SECS: u64 = 60;

/// Per-token contact reveal rate: 10 per minute
pub const CONTACT_REVEAL_RATE_MAX: usize = 10;
pub const CONTACT_REVEAL_RATE_WINDOW_SECS: u64 = 60;

/// New seller daily listing creation limit
pub const NEW_SELLER_DAILY_MAX: i32 = 3;

/// New seller hourly listing creation limit
pub const NEW_SELLER_HOURLY_MAX: i32 = 1;

pub fn is_new_seller(trust_level: &str) -> bool {
    matches!(trust_level, "new" | "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_allows_within_limit() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..5 {
            assert!(limiter.check("test-key", 5, 60));
        }
    }

    #[test]
    fn rate_limiter_denies_excess() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..5 {
            limiter.check("test-key", 5, 60);
        }
        assert!(!limiter.check("test-key", 5, 60));
    }

    #[test]
    fn rate_limiter_allows_different_keys() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..5 {
            limiter.check("key-a", 5, 60);
        }
        assert!(limiter.check("key-b", 5, 60));
    }

    #[test]
    fn rate_limiter_recovers_after_window() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..5 {
            limiter.check("test-key", 5, 1);
        }
        assert!(!limiter.check("test-key", 5, 1));
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(limiter.check("test-key", 5, 1));
    }

    #[test]
    fn rate_limiter_partial_window_cleanup() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..3 {
            limiter.check("test-key", 5, 2);
        }
        std::thread::sleep(std::time::Duration::from_millis(1100));
        // Should retain 3, allow 2 more
        for _ in 0..2 {
            assert!(limiter.check("test-key", 5, 2));
        }
        assert!(!limiter.check("test-key", 5, 2));
    }

    #[test]
    fn rate_limiter_empty_key_allowed() {
        let limiter = SlidingWindowRateLimiter::new();
        assert!(limiter.check("", 1, 60));
        assert!(!limiter.check("", 1, 60));
    }

    #[test]
    fn rate_limiter_zero_max_count() {
        let limiter = SlidingWindowRateLimiter::new();
        assert!(!limiter.check("test-key", 0, 60));
    }

    #[test]
    fn rate_limiter_zero_window() {
        let limiter = SlidingWindowRateLimiter::new();
        assert!(limiter.check("test-key", 1, 0));
        // Window of 0 should not retain timestamps, so always allow
        assert!(limiter.check("test-key", 1, 0));
    }

    #[test]
    fn is_new_seller_checks() {
        assert!(is_new_seller("new"));
        assert!(is_new_seller(""));
        assert!(!is_new_seller("basic"));
        assert!(!is_new_seller("premium"));
        assert!(!is_new_seller("trusted"));
    }
}
