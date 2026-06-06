use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::ledger::{NewTransaction, TransactionType};
use crate::services::ledger_cache::LedgerCache;

use super::super::driver::{BenchError, BenchmarkDriver};

/// Benchmark driver that exercises the in-memory [`LedgerCache`].
///
/// Each operation performs a cache `get_balance` followed by an
/// `apply_transaction` (deposit of 1 credit), measuring the round-trip
/// latency of the full cache path.
pub struct CacheDriver {
    cache: Arc<LedgerCache>,
    agent_id: String,
}

impl CacheDriver {
    pub fn new(cache: Arc<LedgerCache>) -> Self {
        Self {
            cache,
            agent_id: "bench-agent".to_string(),
        }
    }
}

#[async_trait]
impl BenchmarkDriver for CacheDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        // Warm the cache by reading once — subsequent calls are cache hits
        self.cache
            .get_balance(&self.agent_id)
            .await
            .map_err(|e| BenchError::Execution(e.to_string()))?;
        Ok(())
    }

    async fn run_operation(&self) -> Result<Duration, BenchError> {
        let start = std::time::Instant::now();

        // Read from cache
        let _account = self
            .cache
            .get_balance(&self.agent_id)
            .await
            .map_err(|e| BenchError::Db(e.to_string()))?;

        // Write through cache
        let tx = NewTransaction {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            amount: Decimal::new(1, 4), // 0.0001 credits
            tx_type: TransactionType::Deposit,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        let _ = self
            .cache
            .apply_transaction(&tx)
            .await
            .map_err(|e| BenchError::Db(e.to_string()))?;

        Ok(start.elapsed())
    }

    async fn teardown(&self) -> Result<(), BenchError> {
        self.cache.invalidate(&self.agent_id);
        Ok(())
    }
}
