use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Database error: {0}")]
    Db(String),
    #[error("Driver execution error: {0}")]
    Execution(String),
}

/// Asynchronous trait for pluggable benchmark target drivers.
///
/// Each driver encapsulates a single benchmark target (Postgres, Cache, WAL, SSE, HTTP).
/// The lifecycle is: `setup` → `run_operation` (called repeatedly) → `teardown`.
#[async_trait::async_trait]
pub trait BenchmarkDriver: Send + Sync {
    /// Initialize connections, create files, or pre-seed test datasets.
    async fn setup(&self) -> Result<(), BenchError>;

    /// Run a single load operation, returning the elapsed execution duration.
    async fn run_operation(&self) -> Result<Duration, BenchError>;

    /// Clean up files, close connections, or truncate tables.
    async fn teardown(&self) -> Result<(), BenchError>;
}
