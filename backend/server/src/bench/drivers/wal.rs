use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use super::super::driver::{BenchError, BenchmarkDriver};

/// Benchmark driver that simulates write-ahead log (WAL) operations.
///
/// Each operation writes a serialized transaction record to a temp file
/// and calls `sync_all()` (fsync), measuring the full disk I/O latency
/// including cache flush.
pub struct WalDriver {
    pub temp_dir: PathBuf,
    pub file: Mutex<Option<File>>,
}

impl WalDriver {
    pub fn new(temp_dir: PathBuf) -> Self {
        Self {
            temp_dir,
            file: Mutex::new(None),
        }
    }
}

#[async_trait]
impl BenchmarkDriver for WalDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        tokio::fs::create_dir_all(&self.temp_dir)
            .await
            .map_err(|e| BenchError::Io(e.to_string()))?;
        let path = self.temp_dir.join("bench.wal");
        let f = File::create(&path)
            .await
            .map_err(|e| BenchError::Io(e.to_string()))?;
        *self.file.lock().await = Some(f);
        Ok(())
    }

    async fn run_operation(&self) -> Result<Duration, BenchError> {
        let start = std::time::Instant::now();
        let payload =
            b"{\"agent_id\":\"3fa85f64-5717-4562-b3fc-2c963f66afa6\",\"amount\":\"15.5\"}\n";

        let mut lock = self.file.lock().await;
        if let Some(ref mut f) = *lock {
            f.write_all(payload)
                .await
                .map_err(|e| BenchError::Io(e.to_string()))?;
            // Force disk sync (fsync)
            f.sync_all()
                .await
                .map_err(|e| BenchError::Io(e.to_string()))?;
        } else {
            return Err(BenchError::Execution("File not initialized".into()));
        }
        Ok(start.elapsed())
    }

    async fn teardown(&self) -> Result<(), BenchError> {
        // Close the file handle
        *self.file.lock().await = None;
        // Remove the temp file
        let path = self.temp_dir.join("bench.wal");
        if path.exists() {
            tokio::fs::remove_file(path)
                .await
                .map_err(|e| BenchError::Io(e.to_string()))?;
        }
        // Remove the temp directory
        tokio::fs::remove_dir_all(&self.temp_dir)
            .await
            .map_err(|e| BenchError::Io(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wal_driver_writes_and_cleans_up() {
        let temp_dir =
            std::env::temp_dir().join(format!("bench_wal_test_{}", uuid::Uuid::new_v4()));
        let driver = WalDriver::new(temp_dir.clone());

        // Setup should create the temp dir and file
        driver.setup().await.expect("setup should succeed");

        let path = temp_dir.join("bench.wal");
        assert!(path.exists(), "WAL file should exist after setup");

        // Run operation should write data
        let duration = driver.run_operation().await.expect("run should succeed");
        assert!(
            duration.as_micros() > 0,
            "WAL write should take non-zero time"
        );

        // Verify file has content
        let contents = tokio::fs::read_to_string(&path)
            .await
            .expect("should read WAL file");
        assert!(!contents.is_empty(), "WAL file should have content");

        // Teardown should clean up
        driver.teardown().await.expect("teardown should succeed");
        assert!(!path.exists(), "WAL file should be deleted after teardown");
        assert!(
            !temp_dir.exists(),
            "Temp dir should be deleted after teardown"
        );
    }

    #[tokio::test]
    async fn wal_driver_multiple_ops() {
        let temp_dir =
            std::env::temp_dir().join(format!("bench_wal_test_multi_{}", uuid::Uuid::new_v4()));
        let driver = WalDriver::new(temp_dir.clone());

        driver.setup().await.expect("setup");

        for i in 0..10 {
            let result = driver.run_operation().await;
            assert!(result.is_ok(), "op {i} should succeed");
        }

        let path = temp_dir.join("bench.wal");
        let contents = tokio::fs::read_to_string(&path).await.unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 10, "should have 10 WAL entries");

        driver.teardown().await.expect("teardown");
    }
}
