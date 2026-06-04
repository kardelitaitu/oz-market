use metrics::counter;
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub allowed: bool,
    pub remaining: usize,
    pub limit: usize,
    pub reset_after_secs: u64,
}

impl RateLimitStatus {
    pub fn headers(&self) -> [(&'static str, String); 3] {
        [
            ("X-RateLimit-Limit", self.limit.to_string()),
            ("X-RateLimit-Remaining", self.remaining.to_string()),
            ("X-RateLimit-Reset", self.reset_after_secs.to_string()),
        ]
    }
}

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

    /// Check whether `key` is allowed `max_count` times per `window_secs`.
    /// Returns a `RateLimitStatus` with remaining budget and reset time.
    /// When allowed, the current timestamp is recorded into the window.
    pub fn check(&self, key: &str, max_count: usize, window_secs: u64) -> RateLimitStatus {
        let now = std::time::Instant::now();
        let window = std::time::Duration::from_secs(window_secs);
        let mut buckets = self.buckets.write().expect("rate limiter lock");

        let timestamps = buckets.entry(key.to_string()).or_default();

        // Prune expired timestamps
        timestamps.retain(|&t| now.duration_since(t) < window);

        let active_count = timestamps.len();

        // Calculate remaining (before we add the new one, so it's "remaining after this request if allowed")
        let remaining = if active_count >= max_count {
            0
        } else {
            max_count - active_count - 1
        };

        // Calculate reset: time until the oldest remaining timestamp expires,
        // or the full window if no timestamps remain
        let reset_after_secs = if active_count == 0 {
            window_secs
        } else {
            let oldest = timestamps[0];
            let elapsed = now.duration_since(oldest).as_secs();
            window_secs.saturating_sub(elapsed)
        };

        if active_count >= max_count {
            let status = RateLimitStatus {
                allowed: false,
                remaining: 0,
                limit: max_count,
                reset_after_secs,
            };
            counter!("rate_limit_exhausted_total").increment(1);
            warn!(
                action = "rate_limit_exhausted",
                key = %key,
                limit = max_count,
                window_secs = window_secs,
                remaining = 0,
                reset_after_secs = reset_after_secs,
                "Rate limit exhausted for {key}: {}/{} per {window_secs}s",
                active_count, max_count,
            );
            return status;
        }

        timestamps.push(now);

        let status = RateLimitStatus {
            allowed: true,
            remaining,
            limit: max_count,
            reset_after_secs,
        };

        // Warn when a request consumes the last slot (remaining 0 after this request)
        if remaining == 0 {
            warn!(
                action = "rate_limit_last_slot",
                key = %key,
                limit = max_count,
                window_secs = window_secs,
                remaining = 0,
                reset_after_secs = reset_after_secs,
                "Rate limit last slot consumed for {key}: {}/{} per {window_secs}s (next request will be denied)",
                active_count + 1, max_count,
            );
        } else if remaining <= 2 {
            // Close to exhausted: emit a structured log for monitoring aggregation
            warn!(
                action = "rate_limit_near_exhausted",
                key = %key,
                limit = max_count,
                window_secs = window_secs,
                remaining = remaining,
                reset_after_secs = reset_after_secs,
                "Rate limit nearing exhaustion for {key}: {}/{} remaining out of {} per {window_secs}s",
                remaining, max_count, max_count,
            );
        }

        status
    }

    /// Return a snapshot of all current rate limit buckets (active timestamps only).
    /// Prunes expired entries before capture.
    pub fn snapshot(&self) -> Vec<RateLimitBucketSnapshot> {
        let now = std::time::Instant::now();
        let buckets = self.buckets.read().expect("rate limiter lock");
        let mut result = Vec::new();
        for (key, timestamps) in buckets.iter() {
            // Filter to active entries only
            let active: Vec<_> = timestamps
                .iter()
                .filter(|&&t| now.duration_since(t) < std::time::Duration::from_secs(3600)) // 1h max window
                .collect();
            if active.is_empty() {
                continue;
            }
            let count = active.len();
            let oldest = active[0];
            let oldest_age_secs = now.duration_since(*oldest).as_secs();
            result.push(RateLimitBucketSnapshot {
                key: key.clone(),
                count,
                oldest_age_secs,
            });
        }
        // Sort so most active buckets appear first
        result.sort_by(|a, b| b.count.cmp(&a.count));
        result
    }
}

/// Snapshot of a single rate limit bucket (for admin endpoint).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitBucketSnapshot {
    pub key: String,
    pub count: usize,
    pub oldest_age_secs: u64,
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

/// Per-token agent query rate: 20 per minute
pub const AGENT_QUERY_RATE_MAX: usize = 20;
pub const AGENT_QUERY_RATE_WINDOW_SECS: u64 = 60;

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
        for i in 0..5 {
            let status = limiter.check("test-key", 5, 60);
            assert!(status.allowed, "iteration {i} should be allowed");
            assert_eq!(status.remaining, 4 - i, "remaining at iteration {i}");
        }
    }

    #[test]
    fn rate_limiter_denies_excess() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..5 {
            limiter.check("test-key", 5, 60);
        }
        let status = limiter.check("test-key", 5, 60);
        assert!(!status.allowed);
        assert_eq!(status.remaining, 0);
    }

    #[test]
    fn rate_limiter_returns_reset_after_exhaustion() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..5 {
            limiter.check("test-key", 5, 60);
        }
        let status = limiter.check("test-key", 5, 60);
        assert!(!status.allowed);
        assert!(status.reset_after_secs > 0 && status.reset_after_secs <= 60);
    }

    #[test]
    fn rate_limiter_allows_different_keys() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..5 {
            limiter.check("key-a", 5, 60);
        }
        let status = limiter.check("key-b", 5, 60);
        assert!(status.allowed);
        assert_eq!(status.remaining, 4);
    }

    #[test]
    fn rate_limiter_recovers_after_window() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..5 {
            limiter.check("test-key", 5, 1);
        }
        assert!(!limiter.check("test-key", 5, 1).allowed);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let status = limiter.check("test-key", 5, 1);
        assert!(status.allowed);
        assert_eq!(status.remaining, 4);
    }

    #[test]
    fn rate_limiter_partial_window_cleanup() {
        let limiter = SlidingWindowRateLimiter::new();
        for _ in 0..3 {
            limiter.check("test-key", 5, 2);
        }
        std::thread::sleep(std::time::Duration::from_millis(1100));
        // Should retain 3, allow 2 more
        for i in 0..2 {
            let status = limiter.check("test-key", 5, 2);
            assert!(status.allowed, "iteration {i}");
        }
        let status = limiter.check("test-key", 5, 2);
        assert!(!status.allowed);
    }

    #[test]
    fn rate_limiter_empty_key_allowed() {
        let limiter = SlidingWindowRateLimiter::new();
        let status = limiter.check("", 1, 60);
        assert!(status.allowed);
        assert_eq!(status.remaining, 0);
        assert!(!limiter.check("", 1, 60).allowed);
    }

    #[test]
    fn rate_limiter_zero_max_count() {
        let limiter = SlidingWindowRateLimiter::new();
        let status = limiter.check("test-key", 0, 60);
        assert!(!status.allowed);
        assert_eq!(status.remaining, 0);
    }

    #[test]
    fn rate_limiter_zero_window() {
        let limiter = SlidingWindowRateLimiter::new();
        let status = limiter.check("test-key", 1, 0);
        // With zero window, no timestamps are retained, so always allowed
        assert!(status.allowed);
    }

    #[test]
    fn rate_limiter_headers_are_correct() {
        let limiter = SlidingWindowRateLimiter::new();
        let status = limiter.check("test-key", 10, 60);
        assert!(status.allowed);
        let headers = status.headers();
        assert_eq!(headers[0].1, "10");
        assert_eq!(headers[1].1, "9");
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
