use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hdrhistogram::serialization::{Deserializer, Serializer, V2Serializer};
use hdrhistogram::Histogram;
use tokio::sync::{Mutex, Notify};
use tonic::{transport::Server, Request, Response, Status, Streaming};

use super::driver::BenchmarkDriver;
use super::scheduler;

pub mod proto {
    tonic::include_proto!("benchmark");
}

use proto::coordinator_client::CoordinatorClient;
use proto::coordinator_server::{Coordinator, CoordinatorServer};
use proto::{
    MetricFrame, MetricResult, RegisterRequest, RegisterResponse, SyncRequest, SyncResponse,
};

// ─── Coordinator State ──────────────────────────────────────────────

struct CoordinatorState {
    workers: HashMap<String, WorkerInfo>,
    histogram: Histogram<u64>,
    start_time_epoch_ms: i64,
    ready_to_start: Arc<Notify>,
}

struct WorkerInfo {
    #[allow(dead_code)]
    address: String,
    #[allow(dead_code)]
    registered_at: SystemTime,
}

// ─── gRPC Coordinator Implementation ───────────────────────────────

#[derive(Clone)]
pub struct CoordinatorService {
    state: Arc<Mutex<CoordinatorState>>,
}

impl Default for CoordinatorService {
    fn default() -> Self {
        Self::new()
    }
}

impl CoordinatorService {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(CoordinatorState {
            workers: HashMap::new(),
            histogram: Histogram::<u64>::new_with_bounds(1, 60_000_000, 3)
                .expect("valid histogram bounds"),
            start_time_epoch_ms: 0,
            ready_to_start: Arc::new(Notify::new()),
        }));
        Self { state }
    }

    /// Wait until all workers are registered and start time is set.
    pub async fn wait_for_workers(&self, expected: usize) {
        loop {
            let count = self.state.lock().await.workers.len();
            if count >= expected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Set the synchronized start time and notify workers.
    pub async fn set_start_time(&self, epoch_ms: i64) {
        self.state.lock().await.start_time_epoch_ms = epoch_ms;
        self.state.lock().await.ready_to_start.notify_waiters();
    }

    /// Consume and return the merged histogram from all workers.
    pub async fn take_histogram(&self) -> Histogram<u64> {
        let mut state = self.state.lock().await;
        std::mem::replace(
            &mut state.histogram,
            Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).expect("valid histogram bounds"),
        )
    }
}

#[tonic::async_trait]
impl Coordinator for CoordinatorService {
    async fn register_worker(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let assigned_id = format!("worker-{}", uuid::Uuid::new_v4());

        let mut state = self.state.lock().await;
        state.workers.insert(
            assigned_id.clone(),
            WorkerInfo {
                address: req.worker_address,
                registered_at: SystemTime::now(),
            },
        );

        println!("[coordinator] Worker registered: {assigned_id}");
        Ok(Response::new(RegisterResponse {
            accepted: true,
            assigned_id,
        }))
    }

    async fn sync_start(
        &self,
        request: Request<SyncRequest>,
    ) -> Result<Response<SyncResponse>, Status> {
        let req = request.into_inner();
        let state = self.state.lock().await;

        // Wait until coordinator sets the start time
        let notify = state.ready_to_start.clone();
        drop(state);
        notify.notified().await;

        let state = self.state.lock().await;
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let drift = (now_epoch - state.start_time_epoch_ms).abs();
        if drift > 100 {
            return Err(Status::deadline_exceeded(format!(
                "clock drift too high: {drift}ms (max 100ms)"
            )));
        }

        println!(
            "[coordinator] Sync OK for {} | drift: {drift}ms",
            req.config_yaml
        );
        Ok(Response::new(SyncResponse { ready: true }))
    }

    async fn stream_metrics(
        &self,
        request: Request<Streaming<MetricFrame>>,
    ) -> Result<Response<MetricResult>, Status> {
        let mut stream = request.into_inner();

        while let Some(frame) = stream.message().await? {
            let mut state = self.state.lock().await;

            match Deserializer::new().deserialize(&mut &frame.serialized_histogram[..]) {
                Ok(remote_hist) => {
                    if let Err(e) = state.histogram.add(remote_hist) {
                        eprintln!("[coordinator] histogram merge error: {e}");
                    } else {
                        println!(
                            "[coordinator] Received metrics from {} seq={}",
                            frame.worker_id, frame.sequence_number
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[coordinator] deserialize error from {}: {e}",
                        frame.worker_id
                    );
                }
            }
        }

        Ok(Response::new(MetricResult { success: true }))
    }
}

// ─── Worker Client ──────────────────────────────────────────────────

pub struct WorkerClient {
    endpoint: String,
    worker_id: String,
    client: Option<CoordinatorClient<tonic::transport::Channel>>,
}

impl WorkerClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            worker_id: String::new(),
            client: None,
        }
    }

    /// Connect to the coordinator, register, and sync start time.
    pub async fn connect_and_sync(&mut self) -> Result<i64, String> {
        let mut client = CoordinatorClient::connect(self.endpoint.clone())
            .await
            .map_err(|e| format!("connect failed: {e}"))?;

        // Register
        let resp = client
            .register_worker(RegisterRequest {
                worker_id: "worker".to_string(),
                worker_address: "unknown".to_string(),
            })
            .await
            .map_err(|e| format!("register failed: {e}"))?;
        self.worker_id = resp.into_inner().assigned_id;

        // Sync start time
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let sync_resp = client
            .sync_start(SyncRequest {
                config_yaml: "benchmark-run".to_string(),
                start_time_epoch_ms: now_epoch,
            })
            .await
            .map_err(|e| format!("sync failed: {e}"))?;

        if !sync_resp.into_inner().ready {
            return Err("sync returned not ready".to_string());
        }

        self.client = Some(client);
        Ok(now_epoch)
    }

    /// Run the benchmark loop, streaming histograms to the coordinator.
    pub async fn run_and_stream(
        &self,
        driver: Arc<dyn BenchmarkDriver>,
        rate_qps: u64,
        duration: Duration,
        concurrency: usize,
    ) -> Result<(), String> {
        let client = self
            .client
            .as_ref()
            .ok_or("not connected — call connect_and_sync first")?;

        // Run the scheduler — returns the recorded histogram
        let (hist, _error_count) =
            scheduler::run_rate_loop(driver, rate_qps, duration, concurrency).await;

        // Serialize and send the histogram
        let mut serializer = V2Serializer::new();
        let mut buf = Vec::new();
        serializer
            .serialize(&hist, &mut buf)
            .map_err(|e| format!("serialize failed: {e}"))?;

        let mut streaming_client = client.clone();
        let frame = MetricFrame {
            worker_id: self.worker_id.clone(),
            sequence_number: 1,
            serialized_histogram: buf,
        };

        let result = streaming_client
            .stream_metrics(futures::stream::once(async move { frame }))
            .await
            .map_err(|e| format!("stream failed: {e}"))?;

        println!(
            "[worker] Stream result: success={}",
            result.into_inner().success
        );
        Ok(())
    }
}

// ─── Coordinator Runner ─────────────────────────────────────────────

/// Start the gRPC coordinator server in the background and wait for workers.
pub async fn run_coordinator(
    addr: SocketAddr,
    expected_workers: usize,
) -> Result<Arc<CoordinatorService>, String> {
    let service = Arc::new(CoordinatorService::new());

    println!("[coordinator] Starting gRPC server on {addr}");
    println!("[coordinator] Waiting for {expected_workers} workers...");

    // Spawn the gRPC server as a background task so it stays alive
    // to receive streamed metrics after workers register and sync.
    // tonic requires the service to be Clone for the server machinery.
    let server_service = CoordinatorService::clone(&service);
    tokio::spawn(
        Server::builder()
            .add_service(CoordinatorServer::new(server_service))
            .serve(addr),
    );

    // Small delay to let the server start listening
    tokio::time::sleep(Duration::from_millis(100)).await;

    service.wait_for_workers(expected_workers).await;

    // Set synchronized start time
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    service.set_start_time(now_epoch).await;

    println!("[coordinator] All {expected_workers} workers registered. Starting benchmark...");
    Ok(service)
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_service_creation() {
        let service = CoordinatorService::new();
        let state = service.state.lock().await;
        assert!(state.workers.is_empty());
        assert_eq!(state.start_time_epoch_ms, 0);
    }

    #[tokio::test]
    async fn test_histogram_serialization_roundtrip() {
        let mut hist = Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).unwrap();
        hist.record(100).unwrap();
        hist.record(200).unwrap();
        hist.record(300).unwrap();

        let mut serializer = V2Serializer::new();
        let mut buf = Vec::new();
        serializer.serialize(&hist, &mut buf).unwrap();

        let mut deserializer = Deserializer::new();
        let deserialized: Histogram<u64> = deserializer.deserialize(&mut &buf[..]).unwrap();

        assert_eq!(hist.len(), deserialized.len());
        assert_eq!(hist.min(), deserialized.min());
        assert_eq!(hist.max(), deserialized.max());
    }

    #[tokio::test]
    async fn test_histogram_merge() {
        let mut target = Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).unwrap();
        target.record(100).unwrap();
        target.record(200).unwrap();

        let mut source = Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).unwrap();
        source.record(300).unwrap();

        target.add(source).unwrap();

        assert_eq!(target.len(), 3);
        assert_eq!(target.min(), 100);
        assert_eq!(target.max(), 300);
    }
}
