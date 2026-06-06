use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use tokio::sync::Mutex;

use super::super::driver::{BenchError, BenchmarkDriver};

/// Benchmark driver that measures SSE event stream propagation latency.
///
/// The driver subscribes to the server's SSE commit stream, then sends a
/// mock commit event via the internal mock endpoint, measuring how long
/// it takes for the event to arrive on the stream.
pub struct SseDriver {
    base_url: String,
    client: Client,
    stream_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    received_signal: std::sync::Arc<tokio::sync::Notify>,
}

impl SseDriver {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
            stream_task: Mutex::new(None),
            received_signal: std::sync::Arc::new(tokio::sync::Notify::new()),
        }
    }
}

#[async_trait]
impl BenchmarkDriver for SseDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        let sse_url = format!("{}/v1/events/commits", self.base_url);
        let client = self.client.clone();
        let signal = self.received_signal.clone();
        let latency_store = Mutex::new(None::<Duration>);

        // Spawn a background task that subscribes to SSE
        let handle = tokio::spawn(async move {
            let res = match client.get(&sse_url).send().await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("SSE subscribe failed: {e}");
                    return;
                }
            };

            let mut stream = res.bytes_stream();
            let start_wait = std::time::Instant::now();

            while let Some(chunk) = stream.next().await {
                if let Ok(chunk_bytes) = chunk {
                    let text = String::from_utf8_lossy(&chunk_bytes);
                    if text.contains("commit_block") || text.contains("data:") {
                        let latency = start_wait.elapsed();
                        *latency_store.lock().await = Some(latency);
                        signal.notify_one();
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        *self.stream_task.lock().await = Some(handle);

        // Small delay to ensure subscription is established
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    async fn run_operation(&self) -> Result<Duration, BenchError> {
        let mock_url = format!("{}/internal/v1/commits/mock", self.base_url);
        let signal = self.received_signal.clone();

        let start = std::time::Instant::now();

        // Trigger a mock commit event
        let payload = serde_json::json!({
            "item": "Benchmark Item",
            "price": 99.99_f64
        });

        let resp = self
            .client
            .post(&mock_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| BenchError::Execution(format!("mock trigger failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(BenchError::Execution(format!(
                "mock trigger returned {}",
                resp.status()
            )));
        }

        // Wait for the SSE event to arrive (with timeout)
        let timeout = Duration::from_secs(5);
        tokio::select! {
            _ = signal.notified() => {
                Ok(start.elapsed())
            }
            _ = tokio::time::sleep(timeout) => {
                Err(BenchError::Execution("SSE event not received within timeout".into()))
            }
        }
    }

    async fn teardown(&self) -> Result<(), BenchError> {
        if let Some(handle) = self.stream_task.lock().await.take() {
            handle.abort();
        }
        Ok(())
    }
}
