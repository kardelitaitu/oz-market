use crate::client::rate_limit::RateLimitTracker;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub client: reqwest::Client,
    pub base_url: Arc<RwLock<String>>,
    pub rate_limiter: Arc<RwLock<RateLimitTracker>>,
    pub negotiation_listeners: Arc<RwLock<HashMap<String, Arc<AtomicBool>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: Arc::new(RwLock::new("http://127.0.0.1:3000".to_string())),
            rate_limiter: Arc::new(RwLock::new(RateLimitTracker::new())),
            negotiation_listeners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Construct an `AppState` with a custom HTTP client. Used by tests to
    /// bypass system `HTTP_PROXY` env vars (which would otherwise intercept
    /// 127.0.0.1 mock-server traffic and return 403).
    #[cfg(test)]
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            base_url: Arc::new(RwLock::new("http://127.0.0.1:3000".to_string())),
            rate_limiter: Arc::new(RwLock::new(RateLimitTracker::new())),
            negotiation_listeners: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
