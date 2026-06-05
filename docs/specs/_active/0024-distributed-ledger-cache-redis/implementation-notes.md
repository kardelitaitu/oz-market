# Implementation Notes - Distributed Ledger Cache Synchronization

## Distributed Cache Service Design

Below is the design for the Redis-backed cache invalidation service:

```rust
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

pub struct DistributedLedgerCache {
    redis_client: ConnectionManager,
    db_repo: Arc<dyn CreditLedgerRepository>,
}

impl DistributedLedgerCache {
    pub fn new(redis_client: ConnectionManager, db_repo: Arc<dyn CreditLedgerRepository>) -> Self {
        Self {
            redis_client,
            db_repo,
        }
    }

    /// Retrieve balance from Redis or fallback to Postgres
    pub async fn get_balance(&self, agent_id: &Uuid) -> Result<Decimal, String> {
        let key = format!("ledger:balance:{}", agent_id);
        let mut conn = self.redis_client.clone();
        
        // 1. Try to read from Redis
        if let Ok(val) = conn.get::<_, String>(&key).await {
            if let Ok(dec) = val.parse::<Decimal>() {
                return Ok(dec);
            }
        }

        // 2. Fallback to PostgreSQL
        let balance = self.db_repo
            .get_balance(agent_id)
            .await
            .map_err(|e| e.to_string())?;

        // 3. Populate Redis asynchronously with TTL
        let _: Result<(), _> = conn.set_ex(&key, balance.to_string(), 3600).await;
        
        Ok(balance)
    }

    /// Evicts key locally and broadcasts invalidation
    pub async fn invalidate(&self, agent_id: &Uuid) -> Result<(), String> {
        let key = format!("ledger:balance:{}", agent_id);
        let mut conn = self.redis_client.clone();
        
        // Evict from Redis
        let _: Result<(), _> = conn.del(&key).await;
        
        // Broadcast to peer server instances
        let _: Result<(), _> = conn.publish("ledger:invalidation", agent_id.to_string()).await;
        
        Ok(())
    }
}
```
