#![allow(dead_code)]
mod error;
pub mod rate_limit;
pub use error::ApiError;

use marketplace_api_contract::{
    AgentQueryRequest, AgentQueryResponse, CreateListingRequest, CreateListingResponse,
    NegotiationResponse, SearchRequest, SearchResponse,
};
use serde_json::Value;
use std::time::Duration;

use crate::auth::Claims;
use crate::client::rate_limit::RateLimitTracker;

const CLAIMS_HEADER: &str = "x-marketplace-claims";

const RETRY_MAX_ATTEMPTS: u32 = 3;
const RETRY_BASE_MS: u64 = 500;

async fn with_retry<T, F, Fut>(f: F) -> Result<T, ApiError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    let mut last_error = None;
    for attempt in 0..RETRY_MAX_ATTEMPTS {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt + 1 < RETRY_MAX_ATTEMPTS && is_retryable(&e) {
                    let delay = RETRY_BASE_MS * (1u64 << attempt);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    last_error = Some(e);
                } else {
                    return Err(e);
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| ApiError::Status(0, "retry failed".into())))
}

fn is_retryable(error: &ApiError) -> bool {
    let msg = error.to_string().to_lowercase();
    msg.contains("timed out")
        || msg.contains("econnrefused")
        || msg.contains("econnreset")
        || msg.contains("connection reset")
        || msg.contains("timeout")
}

pub struct ApiClient {
    inner: reqwest::Client,
    base_url: String,
    rate_limiter: std::sync::Arc<tokio::sync::RwLock<RateLimitTracker>>,
}

impl ApiClient {
    pub fn new(
        inner: reqwest::Client,
        base_url: String,
        rate_limiter: std::sync::Arc<tokio::sync::RwLock<RateLimitTracker>>,
    ) -> Self {
        Self {
            inner,
            base_url,
            rate_limiter,
        }
    }

    fn claims_header(&self, claims: &Claims) -> String {
        serde_json::to_string(claims).unwrap_or_default()
    }

    /// Check pre-emptive rate limit state. If `remaining == 0` for this action,
    /// sleep until the rate limit window resets.
    async fn check_rate_limit(&self, action: &str) {
        let delay = {
            let tracker = self.rate_limiter.read().await;
            tracker.wait_if_limited(action)
        };
        if let Some(duration) = delay {
            tokio::time::sleep(duration).await;
        }
    }

    /// Parse `X-RateLimit-*` headers from a backend response and return
    /// `(remaining, limit, reset_after_secs)`.
    fn parse_rate_limit_headers(response: &reqwest::Response) -> Option<(u32, u32, u64)> {
        let limit = response
            .headers()
            .get("X-RateLimit-Limit")?
            .to_str()
            .ok()?
            .parse::<u32>()
            .ok()?;
        let remaining = response
            .headers()
            .get("X-RateLimit-Remaining")?
            .to_str()
            .ok()?
            .parse::<u32>()
            .ok()?;
        let reset_after = response
            .headers()
            .get("X-RateLimit-Reset")?
            .to_str()
            .ok()?
            .parse::<u64>()
            .ok()?;
        Some((remaining, limit, reset_after))
    }

    /// Update the rate limit tracker from a response for the given action.
    async fn update_rate_limit(&self, action: &str, response: &reqwest::Response) {
        if let Some((remaining, limit, reset_after_secs)) = Self::parse_rate_limit_headers(response) {
            let mut tracker = self.rate_limiter.write().await;
            tracker.update(action, remaining, limit, reset_after_secs);
        }
    }

    // -- Health ---

    pub async fn health(&self) -> Result<Value, ApiError> {
        with_retry(|| async {
            let resp = self
                .inner
                .get(format!("{}/health", self.base_url))
                .send()
                .await?;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    // -- Listings ---

    pub async fn get_listing(
        &self,
        claims: &Claims,
        listing_id: &str,
    ) -> Result<CreateListingResponse, ApiError> {
        with_retry(|| async {
            let resp = self
                .inner
                .get(format!("{}/v1/listings/{}", self.base_url, listing_id))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .send()
                .await?;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    pub async fn search_listings(
        &self,
        claims: &Claims,
        request: &SearchRequest,
    ) -> Result<SearchResponse, ApiError> {
        self.check_rate_limit("search").await;
        with_retry(|| async {
            let resp = self
                .inner
                .get(format!("{}/v1/listings/search", self.base_url))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .query(request)
                .send()
                .await?;
            self.update_rate_limit("search", &resp).await;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    pub async fn create_listing(
        &self,
        claims: &Claims,
        request: &CreateListingRequest,
    ) -> Result<CreateListingResponse, ApiError> {
        self.check_rate_limit("create").await;
        with_retry(|| async {
            let resp = self
                .inner
                .post(format!("{}/v1/listings", self.base_url))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .json(request)
                .send()
                .await?;
            self.update_rate_limit("create", &resp).await;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    // -- Negotiations ---

    pub async fn open_negotiation(
        &self,
        claims: &Claims,
        request: &marketplace_api_contract::OpenNegotiationRequest,
    ) -> Result<NegotiationResponse, ApiError> {
        self.check_rate_limit("negotiate").await;
        with_retry(|| async {
            let resp = self
                .inner
                .post(format!("{}/v1/negotiations", self.base_url))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .json(request)
                .send()
                .await?;
            self.update_rate_limit("negotiate", &resp).await;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    pub async fn get_negotiation(
        &self,
        claims: &Claims,
        negotiation_id: &str,
    ) -> Result<NegotiationResponse, ApiError> {
        with_retry(|| async {
            let resp = self
                .inner
                .get(format!(
                    "{}/v1/negotiations/{}",
                    self.base_url, negotiation_id
                ))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .send()
                .await?;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    pub async fn submit_offer(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &marketplace_api_contract::SubmitOfferRequest,
    ) -> Result<NegotiationResponse, ApiError> {
        self.check_rate_limit("offer").await;
        with_retry(|| async {
            let resp = self
                .inner
                .post(format!(
                    "{}/v1/negotiations/{}/offers",
                    self.base_url, negotiation_id
                ))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .json(request)
                .send()
                .await?;
            self.update_rate_limit("offer", &resp).await;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    pub async fn accept_negotiation(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &marketplace_api_contract::AcceptNegotiationRequest,
    ) -> Result<NegotiationResponse, ApiError> {
        self.check_rate_limit("accept").await;
        with_retry(|| async {
            let resp = self
                .inner
                .post(format!(
                    "{}/v1/negotiations/{}/accept",
                    self.base_url, negotiation_id
                ))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .json(request)
                .send()
                .await?;
            self.update_rate_limit("accept", &resp).await;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    pub async fn reject_negotiation(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &marketplace_api_contract::RejectNegotiationRequest,
    ) -> Result<NegotiationResponse, ApiError> {
        self.check_rate_limit("reject").await;
        with_retry(|| async {
            let resp = self
                .inner
                .post(format!(
                    "{}/v1/negotiations/{}/reject",
                    self.base_url, negotiation_id
                ))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .json(request)
                .send()
                .await?;
            self.update_rate_limit("reject", &resp).await;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    pub async fn request_contact_reveal(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &marketplace_api_contract::RequestContactRevealRequest,
    ) -> Result<marketplace_api_contract::ContactRevealResponse, ApiError> {
        self.check_rate_limit("reveal").await;
        with_retry(|| async {
            let resp = self
                .inner
                .post(format!(
                    "{}/v1/negotiations/{}/request-contact-reveal",
                    self.base_url, negotiation_id
                ))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .json(request)
                .send()
                .await?;
            self.update_rate_limit("reveal", &resp).await;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    pub async fn approve_contact_reveal(
        &self,
        claims: &Claims,
        reveal_id: &str,
        request: &marketplace_api_contract::RequestContactRevealRequest,
    ) -> Result<marketplace_api_contract::ContactRevealResponse, ApiError> {
        self.check_rate_limit("approve").await;
        with_retry(|| async {
            let resp = self
                .inner
                .post(format!(
                    "{}/v1/contact-reveals/{}/approve",
                    self.base_url, reveal_id
                ))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .json(request)
                .send()
                .await?;
            self.update_rate_limit("approve", &resp).await;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }

    // -- Agent ---

    pub async fn agent_query(
        &self,
        claims: &Claims,
        request: &AgentQueryRequest,
    ) -> Result<AgentQueryResponse, ApiError> {
        self.check_rate_limit("agent").await;
        with_retry(|| async {
            let resp = self
                .inner
                .post(format!("{}/v1/agent/query", self.base_url))
                .header(CLAIMS_HEADER, self.claims_header(claims))
                .json(request)
                .send()
                .await?;
            self.update_rate_limit("agent", &resp).await;
            if !resp.status().is_success() {
                return Err(ApiError::from_response(resp).await);
            }
            Ok(resp.json().await?)
        })
        .await
    }
}
