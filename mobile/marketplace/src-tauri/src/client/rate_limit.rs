use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize)]
pub struct RateLimitEntry {
    pub remaining: u32,
    pub limit: u32,
    pub reset_after_secs: u64,
    #[serde(skip)]
    pub reset_at: Instant,
}

#[derive(Clone, Debug, Serialize)]
pub struct RateLimitSummary {
    pub action: String,
    pub remaining: u32,
    pub limit: u32,
    pub reset_after_secs: u64,
}

/// Tracks per-action rate limits parsed from `X-RateLimit-*` response headers.
/// Used for pre-emptive client-side backoff — when `remaining == 0`, the client
/// waits until `reset_at` before issuing the next request of that action type.
pub struct RateLimitTracker {
    limits: HashMap<String, RateLimitEntry>,
}

impl RateLimitTracker {
    pub fn new() -> Self {
        Self {
            limits: HashMap::new(),
        }
    }

    /// Update tracked state from backend response headers.
    pub fn update(&mut self, action: &str, remaining: u32, limit: u32, reset_after_secs: u64) {
        self.limits.insert(
            action.to_string(),
            RateLimitEntry {
                remaining,
                limit,
                reset_after_secs,
                reset_at: Instant::now() + Duration::from_secs(reset_after_secs),
            },
        );
    }

    /// Returns `Some(Duration)` to wait if this action is currently rate-limited
    /// (remaining == 0 and the window hasn't reset yet). Otherwise `None`.
    pub fn wait_if_limited(&self, action: &str) -> Option<Duration> {
        if let Some(entry) = self.limits.get(action) {
            if entry.remaining == 0 {
                let remaining = entry.reset_at.saturating_duration_since(Instant::now());
                if !remaining.is_zero() {
                    return Some(remaining);
                }
            }
        }
        None
    }

    /// Return all tracked rate limits as serializable summaries.
    pub fn all_limits(&self) -> Vec<RateLimitSummary> {
        self.limits
            .iter()
            .map(|(action, entry)| RateLimitSummary {
                action: action.clone(),
                remaining: entry.remaining,
                limit: entry.limit,
                reset_after_secs: entry.reset_after_secs,
            })
            .collect()
    }
}
