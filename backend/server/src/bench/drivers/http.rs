use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use super::super::driver::{BenchError, BenchmarkDriver};

/// Benchmark driver that exercises the HTTP API layer.
///
/// Each operation sends a GET request to the server's health endpoint,
/// measuring the full HTTP round-trip latency including routing, middleware,
/// rate limiter, and response serialization.
pub struct HttpDriver {
    base_url: String,
    client: Client,
    target_path: String,
}

impl HttpDriver {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.clone(),
            client: Client::new(),
            target_path: format!("{}/health", base_url),
        }
    }

    /// Create a driver that targets a specific API path.
    pub fn with_path(base_url: String, path: &str) -> Self {
        Self {
            base_url: base_url.clone(),
            client: Client::new(),
            target_path: format!("{}{}", base_url.trim_end_matches('/'), path),
        }
    }
}

#[async_trait]
impl BenchmarkDriver for HttpDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        // Verify the server is reachable
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

        let resp = self
            .client
            .get(&self.target_path)
            .send()
            .await
            .map_err(|e| BenchError::Execution(format!("request failed: {e}")))?;

        // Consume response body to ensure full round-trip
        let _body = resp
            .bytes()
            .await
            .map_err(|e| BenchError::Execution(format!("read body failed: {e}")))?;

        Ok(start.elapsed())
    }

    async fn teardown(&self) -> Result<(), BenchError> {
        // No cleanup needed for HTTP driver
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
    fn test_http_driver_with_path() {
        let driver =
            HttpDriver::with_path("http://127.0.0.1:3000".to_string(), "/v1/listings/search");
        assert_eq!(
            driver.target_path,
            "http://127.0.0.1:3000/v1/listings/search"
        );
    }
}
