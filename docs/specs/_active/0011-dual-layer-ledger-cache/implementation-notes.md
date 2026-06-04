# Implementation Notes - Dual-Layer Ledger Trait and Synchronous Cache

## In-Memory Cache Implementation Details

```rust
use dashmap::DashMap;
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

pub struct LedgerCache {
    balances: DashMap<Uuid, Decimal>,
    db_repo: Arc<dyn CreditLedgerRepository>,
}

impl LedgerCache {
    pub fn new(db_repo: Arc<dyn CreditLedgerRepository>) -> Self {
        Self {
            balances: DashMap::new(),
            db_repo,
        }
    }

    /// Retrieve balance from cache or fall back to DB read.
    pub async fn get_balance(&self, agent_id: &Uuid) -> Result<Decimal, CreditLedgerError> {
        // Look up using DashMap's concurrent shard read lock
        if let Some(entry) = self.balances.get(agent_id) {
            return Ok(*entry.value());
        }

        // Cache miss: query the DB repository
        let balance = self.db_repo.get_balance(agent_id).await?;
        
        // Populate cache and return
        self.balances.insert(*agent_id, balance);
        Ok(balance)
    }

    /// Mutates balance using a synchronous write-through flow
    pub async fn apply_transaction(
        &self,
        tx: &NewTransaction,
    ) -> Result<Decimal, CreditLedgerError> {
        // Step 1: Write to PostgreSQL database first
        let account = match self.db_repo.apply_transaction(tx).await {
            Ok(acc) => acc,
            Err(err) => {
                // Critical: Evict the cache key on failure to ensure we don't hold stale state
                self.balances.remove(&tx.agent_id);
                return Err(err);
            }
        };

        // Step 2: On DB success, update in-memory cache
        self.balances.insert(tx.agent_id, account.balance);
        Ok(account.balance)
    }

    /// Evicts key from memory cache
    pub fn invalidate(&self, agent_id: &Uuid) {
        self.balances.remove(agent_id);
    }
}
```

## Mock Repository for Cache Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockRepository {
        balance: Mutex<Decimal>,
        should_fail: Mutex<bool>,
    }

    #[async_trait]
    impl CreditLedgerRepository for MockRepository {
        async fn get_balance(&self, _id: &Uuid) -> Result<Decimal, CreditLedgerError> {
            Ok(*self.balance.lock().unwrap())
        }

        async fn apply_transaction(&self, tx: &NewTransaction) -> Result<CreditAccount, CreditLedgerError> {
            if *self.should_fail.lock().unwrap() {
                return Err(CreditLedgerError::DatabaseError("DB Fail".into()));
            }
            let mut bal = self.balance.lock().unwrap();
            *bal += tx.amount;
            Ok(CreditAccount {
                agent_id: tx.agent_id,
                balance: *bal,
            })
        }

        async fn get_transaction_history(&self, _id: &Uuid, _l: usize, _o: usize) -> Result<Vec<CreditTransaction>, CreditLedgerError> {
            Ok(vec![])
        }
    }
}
```
