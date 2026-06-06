use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::HeaderValue;
use reqwest::Client;

use super::super::driver::{BenchError, BenchmarkDriver};

/// HTTP benchmark mode: which endpoint to target and whether to send auth claims.
enum HttpBenchMode {
    /// GET /health — no auth
    Health,
    /// GET /v1/listings/search?query=... — with x-marketplace-claims header
    Search,
    /// GET /v1/listings/{id} — with x-marketplace-claims header
    GetListing,
}

/// Benchmark driver that exercises the HTTP API layer.
///
/// Supports three modes:
/// - `health` (default): GET /health — measures raw HTTP round-trip latency
/// - `search`: authenticated GET /v1/listings/search — measures search endpoint latency
/// - `get-listing`: authenticated GET /v1/listings/{id} — measures listing lookup latency
pub struct HttpDriver {
    base_url: String,
    client: Client,
    mode: HttpBenchMode,
    claims_header: Option<HeaderValue>,
}

impl HttpDriver {
    /// Create a driver that hits the health endpoint (no auth).
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
            mode: HttpBenchMode::Health,
            claims_header: None,
        }
    }

    /// Create a driver that sends authenticated requests to a specific endpoint.
    ///
    /// `mode` is one of `"search"` or `"get-listing"`.
    pub fn with_claims(base_url: String, claims_json: String, mode: &str) -> Self {
        let hc = match mode {
            "get-listing" => HttpBenchMode::GetListing,
            _ => HttpBenchMode::Search,
        };
        Self {
            base_url,
            client: Client::new(),
            mode: hc,
            claims_header: Some(
                HeaderValue::from_str(&claims_json).expect("valid claims header"),
            ),
        }
    }
}

#[async_trait]
impl BenchmarkDriver for HttpDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map_err(|e| BenchError::Execution(format!("server unreachable: {e}")))?;
        if !resp.status().is_success() {
            return Err(BenchError::Execution(format!(
                "health check failed: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn run_operation(&self) -> Result<Duration, BenchError> {
        let start = std::time::Instant::now();

        let url = match self.mode {
            HttpBenchMode::Health => format!("{}/health", self.base_url),
            HttpBenchMode::Search => {
                format!("{}/v1/listings/search?query=benchmark&limit=20", self.base_url)
            }
            HttpBenchMode::GetListing => {
                format!("{}/v1/listings/bench-listing-id", self.base_url)
            }
        };

        let mut req = self.client.get(&url);
        if let Some(ref header) = self.claims_header {
            req = req.header("x-marketplace-claims", header.clone());
        }

        let resp = req
            .send()
            .await
            .map_err(|e| BenchError::Execution(format!("request failed: {e}")))?;

        let _body = resp
            .bytes()
            .await
            .map_err(|e| BenchError::Execution(format!("read body failed: {e}")))?;

        Ok(start.elapsed())
    }

    async fn teardown(&self) -> Result<(), BenchError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_driver_construction() {
        let driver = HttpDriver::new("http://127.0.0.1:3000".to_string());
        assert_eq!(driver.base_url, "http://127.0.0.1:3000");
    }

    #[test]
    fn test_http_driver_with_claims() {
        let claims = r#"{"sub":"test","roles":["admin"],"scopes":["listing:search"]}"#;
        let driver = HttpDriver::with_claims(
            "http://127.0.0.1:3000".to_string(),
            claims.to_string(),
            "search",
        );
        assert!(driver.claims_header.is_some());
    }
}