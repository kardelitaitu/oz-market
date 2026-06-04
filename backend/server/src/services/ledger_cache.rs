use std::sync::Arc;

use dashmap::DashMap;

use crate::domain::ledger::{
    CreditAccount, CreditLedgerError, CreditLedgerRepository, CreditTransaction, NewTransaction,
};

/// Write-through cache over a [`CreditLedgerRepository`].
///
/// Balances are stored in a `DashMap<String, CreditAccount>` keyed by `agent_id`.
/// Reads check the cache first; on miss they query the DB and populate the cache.
/// Writes commit to the DB first and only update the cache on success.
pub struct LedgerCache {
    balances: DashMap<String, CreditAccount>,
    db_repo: Arc<dyn CreditLedgerRepository>,
}

impl LedgerCache {
    pub fn new(db_repo: Arc<dyn CreditLedgerRepository>) -> Self {
        Self {
            balances: DashMap::new(),
            db_repo,
        }
    }

    /// Retrieve the agent's balance from cache, or fall back to the DB.
    pub async fn get_balance(&self, agent_id: &str) -> Result<CreditAccount, CreditLedgerError> {
        if let Some(entry) = self.balances.get(agent_id) {
            return Ok(entry.value().clone());
        }

        let account = self.db_repo.get_balance(agent_id).await?;
        self.balances.insert(agent_id.to_owned(), account.clone());
        Ok(account)
    }

    /// Write-through: DB commit first, then update cache on success.
    ///
    /// On DB failure the cached entry is evicted to prevent stale state.
    pub async fn apply_transaction(
        &self,
        tx: &NewTransaction,
    ) -> Result<CreditAccount, CreditLedgerError> {
        let account = match self.db_repo.apply_transaction(tx).await {
            Ok(acc) => acc,
            Err(err) => {
                self.balances.remove(&tx.agent_id);
                return Err(err);
            }
        };

        self.balances.insert(tx.agent_id.clone(), account.clone());
        Ok(account)
    }

    /// Delegate transaction history queries to the underlying DB repository.
    pub async fn get_transaction_history(
        &self,
        agent_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<CreditTransaction>, CreditLedgerError> {
        self.db_repo
            .get_transaction_history(agent_id, limit, offset)
            .await
    }

    /// Evict a single agent's cached balance entry.
    pub fn invalidate(&self, agent_id: &str) {
        self.balances.remove(agent_id);
    }

    /// Evict all cached balances.
    pub fn invalidate_all(&self) {
        self.balances.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rust_decimal::Decimal;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::domain::ledger::TransactionType;

    struct MockRepo {
        balance: Mutex<Decimal>,
        fail_tx: Mutex<bool>,
        get_balance_calls: AtomicUsize,
        apply_tx_calls: AtomicUsize,
    }

    impl MockRepo {
        fn new(balance: Decimal) -> Self {
            Self {
                balance: Mutex::new(balance),
                fail_tx: Mutex::new(false),
                get_balance_calls: AtomicUsize::new(0),
                apply_tx_calls: AtomicUsize::new(0),
            }
        }

        fn set_fail(&self, fail: bool) {
            *self.fail_tx.lock().unwrap() = fail;
        }
    }

    #[async_trait]
    impl CreditLedgerRepository for MockRepo {
        async fn get_balance(&self, agent_id: &str) -> Result<CreditAccount, CreditLedgerError> {
            self.get_balance_calls.fetch_add(1, Ordering::SeqCst);
            let bal = *self.balance.lock().unwrap();
            Ok(CreditAccount {
                agent_id: agent_id.to_owned(),
                balance_credits: bal,
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
        }

        async fn apply_transaction(
            &self,
            tx: &NewTransaction,
        ) -> Result<CreditAccount, CreditLedgerError> {
            self.apply_tx_calls.fetch_add(1, Ordering::SeqCst);
            if *self.fail_tx.lock().unwrap() {
                return Err(CreditLedgerError::DatabaseError("mock fail".into()));
            }
            let mut bal = self.balance.lock().unwrap();
            *bal += tx.amount;
            Ok(CreditAccount {
                agent_id: tx.agent_id.clone(),
                balance_credits: *bal,
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
        }

        async fn get_transaction_history(
            &self,
            _agent_id: &str,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<CreditTransaction>, CreditLedgerError> {
            Ok(vec![])
        }
    }

    fn make_tx(agent_id: &str, amount: Decimal) -> NewTransaction {
        NewTransaction {
            id: Uuid::new_v4(),
            agent_id: agent_id.to_owned(),
            amount,
            tx_type: TransactionType::Deposit,
            idempotency_key: Uuid::new_v4().to_string(),
        }
    }

    #[tokio::test]
    async fn get_balance_hit_returns_cached() {
        let mock = Arc::new(MockRepo::new(Decimal::new(5000, 4)));
        let cache = LedgerCache::new(mock.clone());

        cache.balances.insert(
            "agent-1".into(),
            CreditAccount {
                agent_id: "agent-1".into(),
                balance_credits: Decimal::new(5000, 4),
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
        );

        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(5000, 4));
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn get_balance_miss_queries_db_and_populates_cache() {
        let mock = Arc::new(MockRepo::new(Decimal::new(3000, 4)));
        let cache = LedgerCache::new(mock.clone());

        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(3000, 4));
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 1);

        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(3000, 4));
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn apply_transaction_updates_cache_on_success() {
        let mock = Arc::new(MockRepo::new(Decimal::ZERO));
        let cache = LedgerCache::new(mock.clone());

        let tx = make_tx("agent-1", Decimal::new(1000, 4));
        let account = cache.apply_transaction(&tx).await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(1000, 4));
        assert_eq!(mock.apply_tx_calls.load(Ordering::SeqCst), 1);

        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(1000, 4));
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn apply_transaction_evicts_cache_on_db_failure() {
        let mock = Arc::new(MockRepo::new(Decimal::new(5000, 4)));
        let cache = LedgerCache::new(mock.clone());

        cache.balances.insert(
            "agent-1".into(),
            CreditAccount {
                agent_id: "agent-1".into(),
                balance_credits: Decimal::new(5000, 4),
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
        );

        mock.set_fail(true);
        let tx = make_tx("agent-1", Decimal::new(1000, 4));
        let result = cache.apply_transaction(&tx).await;
        assert!(result.is_err());

        assert!(cache.balances.get("agent-1").is_none());

        mock.set_fail(false);
        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(5000, 4));
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn get_transaction_history_delegates_to_db() {
        let mock = Arc::new(MockRepo::new(Decimal::ZERO));
        let cache = LedgerCache::new(mock.clone());

        let result = cache.get_transaction_history("agent-1", 10, 0).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn invalidate_removes_from_cache() {
        let mock = Arc::new(MockRepo::new(Decimal::new(5000, 4)));
        let cache = LedgerCache::new(mock.clone());

        let _ = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 1);

        cache.invalidate("agent-1");
        assert!(cache.balances.get("agent-1").is_none());

        let _ = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn concurrent_reads_and_writes() {
        let mock = Arc::new(MockRepo::new(Decimal::ZERO));
        let cache = Arc::new(LedgerCache::new(mock.clone()));

        let mut handles = Vec::new();

        for _ in 0..10 {
            let cache = Arc::clone(&cache);
            let tx = make_tx("agent-1", Decimal::new(100, 4));
            handles.push(tokio::spawn(
                async move { cache.apply_transaction(&tx).await },
            ));
        }

        for h in handles {
            let result = h.await.unwrap();
            assert!(result.is_ok());
        }

        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(1000, 4));

        let db_calls = mock.get_balance_calls.load(Ordering::SeqCst);
        assert_eq!(db_calls, 0);
    }
}
