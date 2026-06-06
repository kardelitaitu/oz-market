use std::time::Duration;

use async_trait::async_trait;
use sqlx::PgPool;

use super::super::driver::{BenchError, BenchmarkDriver};

/// Benchmark driver that exercises a Postgres connection pool.
///
/// Each operation runs a simple `SELECT 1` health query followed by
/// a parameterized INSERT into a benchmark scratch table, measuring
/// end-to-end database round-trip latency.
pub struct PostgresDriver {
    pool: PgPool,
    scratch_table: String,
}

impl PostgresDriver {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            scratch_table: "bench_scratch".to_string(),
        }
    }
}

#[async_trait]
impl BenchmarkDriver for PostgresDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        // Create scratch table for benchmark writes
        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id UUID PRIMARY KEY,
                payload TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )",
            self.scratch_table
        ))
        .execute(&self.pool)
        .await
        .map_err(|e| BenchError::Db(e.to_string()))?;
        Ok(())
    }

    async fn run_operation(&self) -> Result<Duration, BenchError> {
        let start = std::time::Instant::now();

        // Read: simple query to check connection + round-trip
        let _row: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| BenchError::Db(e.to_string()))?;

        // Write: insert a row
        let id = uuid::Uuid::new_v4();
        sqlx::query(&format!(
            "INSERT INTO {} (id, payload) VALUES ($1, $2)",
            self.scratch_table
        ))
        .bind(id)
        .bind("benchmark-payload")
        .execute(&self.pool)
        .await
        .map_err(|e| BenchError::Db(e.to_string()))?;

        Ok(start.elapsed())
    }

    async fn teardown(&self) -> Result<(), BenchError> {
        // Clean up scratch data
        sqlx::query(&format!("DELETE FROM {}", self.scratch_table))
            .execute(&self.pool)
            .await
            .map_err(|e| BenchError::Db(e.to_string()))?;
        Ok(())
    }
}
