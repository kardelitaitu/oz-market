# Implementation Notes - Benchmark Component Drivers

## Driver Trait and WAL Driver Design

Below is the implementation design for the `BenchmarkDriver` trait and the `WalDriver`:

```rust
use async_trait::async_trait;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::path::PathBuf;
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Database error: {0}")]
    Db(String),
    #[error("Driver execution error: {0}")]
    Execution(String),
}

#[async_trait]
pub trait BenchmarkDriver: Send + Sync {
    /// Setup connections, create files, or pre-seed test datasets.
    async fn setup(&self) -> Result<(), BenchError>;

    /// Run a single load operation, returning the elapsed execution duration.
    async fn run_operation(&self) -> Result<Duration, BenchError>;

    /// Clean up files, close connections, or truncate tables.
    async fn teardown(&self) -> Result<(), BenchError>;
}

pub struct WalDriver {
    pub temp_dir: PathBuf,
    pub file: Mutex<Option<File>>,
}

#[async_trait]
impl BenchmarkDriver for WalDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        let path = self.temp_dir.join("bench.wal");
        let f = File::create(&path)
            .await
            .map_err(|e| BenchError::Io(e.to_string()))?;
        *self.file.lock().await = Some(f);
        Ok(())
    }

    async fn run_operation(&self) -> Result<Duration, BenchError> {
        let start = std::time::Instant::now();
        let payload = b"{\"agent_id\":\"3fa85f64-5717-4562-b3fc-2c963f66afa6\",\"amount\":\"15.5\"}\n";
        
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
        *self.file.lock().await = None;
        let path = self.temp_dir.join("bench.wal");
        if path.exists() {
            tokio::fs::remove_file(path)
                .await
                .map_err(|e| BenchError::Io(e.to_string()))?;
        }
        Ok(())
    }
}
```
