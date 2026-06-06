pub mod cache;
pub mod http;
pub mod postgres;
pub mod sse;
pub mod wal;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use sqlx::PgPool;

use super::driver::{BenchError, BenchmarkDriver};
use crate::services::ledger_cache::LedgerCache;

pub use self::cache::CacheDriver;
pub use self::http::HttpDriver;
pub use self::postgres::PostgresDriver;
pub use self::sse::SseDriver;
pub use self::wal::WalDriver;

/// A no-op mock driver that sleeps for a fixed duration.
///
/// Useful for validating the scheduler and histogram recording without
/// requiring any external services.
pub struct MockDriver {
    operation_delay: Duration,
}

impl MockDriver {
    pub fn new(operation_delay: Duration) -> Self {
        Self { operation_delay }
    }
}

#[async_trait]
impl BenchmarkDriver for MockDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        Ok(())
    }

    async fn run_operation(&self) -> Result<Duration, BenchError> {
        let start = std::time::Instant::now();
        tokio::time::sleep(self.operation_delay).await;
        Ok(start.elapsed())
    }

    async fn teardown(&self) -> Result<(), BenchError> {
        Ok(())
    }
}

/// Create a benchmark driver based on the target name.
///
/// Supported targets:
/// - `mock` — no-op mock driver (default)
/// - `cache` — [`LedgerCache`] cache driver
/// - `postgres` — `PgPool` database driver
/// - `wal` — temp file WAL driver
/// - `sse` — SSE event stream driver (requires a running server)
/// - `http` — HTTP API driver (requires a running server)
pub fn create_driver(
    target: &str,
    pool: Option<PgPool>,
    cache: Option<Arc<LedgerCache>>,
    base_url: Option<&str>,
) -> Arc<dyn BenchmarkDriver> {
    match target {
        "cache" => {
            let cache = cache.expect("CacheDriver requires a LedgerCache instance");
            Arc::new(CacheDriver::new(cache))
        }
        "postgres" => {
            let pool = pool.expect("PostgresDriver requires a PgPool instance");
            Arc::new(PostgresDriver::new(pool))
        }
        "wal" => {
            let temp_dir = std::env::temp_dir().join("oz_market_bench_wal");
            Arc::new(WalDriver::new(temp_dir))
        }
        "sse" => {
            let url = base_url.unwrap_or("http://127.0.0.1:3000");
            Arc::new(SseDriver::new(url.to_string()))
        }
        "http" => {
            let url = base_url.unwrap_or("http://127.0.0.1:3000");
            Arc::new(HttpDriver::new(url.to_string()))
        }
        _ => Arc::new(MockDriver::new(Duration::from_micros(100))),
    }
}
