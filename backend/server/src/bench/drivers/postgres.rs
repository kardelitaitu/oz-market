use std::time::Duration;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::driver::{BenchError, BenchmarkDriver};

/// Benchmark driver that exercises a Postgres connection pool.
///
/// ## Normal mode (default)
/// Each operation runs a simple `SELECT 1` health query followed by
/// a parameterized INSERT into a benchmark scratch table, measuring
/// end-to-end database round-trip latency.
///
/// ## Search mode (`with_search`)
/// Setup seeds N listings into a search scratch table; each operation
/// runs a parameterized text-search query; teardown removes the data.
pub struct PostgresDriver {
    pool: PgPool,
    scratch_table: String,
    search_mode: bool,
}

impl PostgresDriver {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            scratch_table: "bench_scratch".to_string(),
            search_mode: false,
        }
    }

    /// Create a driver in search-query mode.
    ///
    /// Each operation measures a `WHERE text ILIKE` search against
    /// pre-seeded benchmark rows.
    pub fn with_search(pool: PgPool) -> Self {
        Self {
            pool,
            scratch_table: "bench_scratch".to_string(),
            search_mode: true,
        }
    }
}

#[async_trait]
impl BenchmarkDriver for PostgresDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        if self.search_mode {
            // Create search scratch table with a GIN index for realistic latency
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS bench_search (
                    id UUID PRIMARY KEY,
                    title TEXT NOT NULL,
                    category TEXT NOT NULL,
                    price DOUBLE PRECISION NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW()
                )",
            )
            .execute(&self.pool)
            .await
            .map_err(|e| BenchError::Db(e.to_string()))?;

            sqlx::query(
                "CREATE INDEX IF NOT EXISTS idx_bench_search_title
                 ON bench_search USING gin(to_tsvector('english', title))",
            )
            .execute(&self.pool)
            .await
            .map_err(|e| BenchError::Db(e.to_string()))?;

            // Seed 1000 benchmark listings
            for i in 0..1000 {
                sqlx::query(
                    "INSERT INTO bench_search (id, title, category, price)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (id) DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(format!("Benchmark Listing {i} — premium quality item for sale"))
                .bind(match i % 3 {
                    0 => "Electronics",
                    1 => "Books",
                    _ => "Home",
                })
                .bind(10.0 + (i % 100) as f64 * 9.99)
                .execute(&self.pool)
                .await
                .map_err(|e| BenchError::Db(e.to_string()))?;
            }
        } else {
            // Normal mode: create scratch table for benchmark writes
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
        }
        Ok(())
    }

    async fn run_operation(&self) -> Result<Duration, BenchError> {
        let start = std::time::Instant::now();

        if self.search_mode {
            // Search: full-text search query mimicking SearchService
            let _rows: Vec<(Uuid, String, String, f64)> = sqlx::query_as(
                "SELECT id, title, category, price
                 FROM bench_search
                 WHERE to_tsvector('english', title) @@ plainto_tsquery('english', $1)
                 ORDER BY created_at DESC
                 LIMIT 20",
            )
            .bind("benchmark premium")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| BenchError::Db(e.to_string()))?;
        } else {
            // Normal mode: SELECT 1 + INSERT
            let _row: (i32,) = sqlx::query_as("SELECT 1")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| BenchError::Db(e.to_string()))?;

            let id = Uuid::new_v4();
            sqlx::query(&format!(
                "INSERT INTO {} (id, payload) VALUES ($1, $2)",
                self.scratch_table
            ))
            .bind(id)
            .bind("benchmark-payload")
            .execute(&self.pool)
            .await
            .map_err(|e| BenchError::Db(e.to_string()))?;
        }

        Ok(start.elapsed())
    }

    async fn teardown(&self) -> Result<(), BenchError> {
        if self.search_mode {
            sqlx::query("DROP TABLE IF EXISTS bench_search")
                .execute(&self.pool)
                .await
                .map_err(|e| BenchError::Db(e.to_string()))?;
        } else {
            sqlx::query(&format!("DELETE FROM {}", self.scratch_table))
                .execute(&self.pool)
                .await
                .map_err(|e| BenchError::Db(e.to_string()))?;
        }
        Ok(())
    }
}
