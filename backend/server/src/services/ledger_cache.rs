use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::domain::ledger::{
    CreditAccount, CreditLedgerError, CreditLedgerRepository, CreditTransaction, NewTransaction,
};

const DEFAULT_TTL_SECS: u64 = 300;

struct CachedEntry {
    account: CreditAccount,
    inserted_at: Instant,
}

/// Write-through cache over a [`CreditLedgerRepository`].
///
/// Balances are stored in a `DashMap<String, CachedEntry>` keyed by `agent_id`.
/// Reads check the cache first; on miss they query the DB and populate the cache.
/// Writes commit to the DB first and only update the cache on success.
///
/// Cache entries have a configurable TTL. Expired entries are treated as cache
/// misses and re-fetched from the DB.
pub struct LedgerCache {
    balances: DashMap<String, CachedEntry>,
    db_repo: Arc<dyn CreditLedgerRepository>,
    ttl: Duration,
}

impl LedgerCache {
    /// Create a new `LedgerCache` with the given TTL.
    ///
    /// Uses `LEDGER_CACHE_TTL_SECS` env var if set, otherwise defaults to 300s.
    pub fn new(db_repo: Arc<dyn CreditLedgerRepository>) -> Self {
        let ttl_secs = std::env::var("LEDGER_CACHE_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_TTL_SECS);
        Self {
            balances: DashMap::new(),
            db_repo,
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Create a cache with an explicit (non-env) TTL. Used in tests.
    pub fn with_ttl(db_repo: Arc<dyn CreditLedgerRepository>, ttl: Duration) -> Self {
        Self {
            balances: DashMap::new(),
            db_repo,
            ttl,
        }
    }

    /// Retrieve the agent's balance from cache, or fall back to the DB.
    ///
    /// Expired cache entries are evicted and treated as a cache miss.
    pub async fn get_balance(&self, agent_id: &str) -> Result<CreditAccount, CreditLedgerError> {
        if let Some(entry) = self.balances.get(agent_id) {
            if entry.inserted_at.elapsed() < self.ttl {
                return Ok(entry.account.clone());
            }
            drop(entry);
            self.balances.remove(agent_id);
        }

        let account = self.db_repo.get_balance(agent_id).await?;
        self.balances.insert(
            agent_id.to_owned(),
            CachedEntry {
                account: account.clone(),
                inserted_at: Instant::now(),
            },
        );
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

        self.balances.insert(
            tx.agent_id.clone(),
            CachedEntry {
                account: account.clone(),
                inserted_at: Instant::now(),
            },
        );
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

    /// A constant-length TTL that will never expire during a test.
    const LONG_TTL: Duration = Duration::from_secs(3600);
    /// A zero-length TTL so the entry is instantly expired.
    const ZERO_TTL: Duration = Duration::from_secs(0);

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
        let cache = LedgerCache::with_ttl(mock.clone(), LONG_TTL);

        cache.balances.insert(
            "agent-1".into(),
            CachedEntry {
                account: CreditAccount {
                    agent_id: "agent-1".into(),
                    balance_credits: Decimal::new(5000, 4),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                },
                inserted_at: Instant::now(),
            },
        );

        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(5000, 4));
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn get_balance_miss_queries_db_and_populates_cache() {
        let mock = Arc::new(MockRepo::new(Decimal::new(3000, 4)));
        let cache = LedgerCache::with_ttl(mock.clone(), LONG_TTL);

        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(3000, 4));
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 1);

        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(3000, 4));
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn get_balance_evicts_expired_entry() {
        let mock = Arc::new(MockRepo::new(Decimal::new(7777, 4)));
        let cache = LedgerCache::with_ttl(mock.clone(), ZERO_TTL);

        cache.balances.insert(
            "agent-1".into(),
            CachedEntry {
                account: CreditAccount {
                    agent_id: "agent-1".into(),
                    balance_credits: Decimal::new(5000, 4),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                },
                inserted_at: Instant::now(),
            },
        );

        let account = cache.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(7777, 4));
        assert_eq!(mock.get_balance_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn apply_transaction_updates_cache_on_success() {
        let mock = Arc::new(MockRepo::new(Decimal::ZERO));
        let cache = LedgerCache::with_ttl(mock.clone(), LONG_TTL);

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
        let cache = LedgerCache::with_ttl(mock.clone(), LONG_TTL);

        cache.balances.insert(
            "agent-1".into(),
            CachedEntry {
                account: CreditAccount {
                    agent_id: "agent-1".into(),
                    balance_credits: Decimal::new(5000, 4),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                },
                inserted_at: Instant::now(),
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
        let cache = LedgerCache::with_ttl(mock.clone(), LONG_TTL);

        let result = cache.get_transaction_history("agent-1", 10, 0).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn invalidate_removes_from_cache() {
        let mock = Arc::new(MockRepo::new(Decimal::new(5000, 4)));
        let cache = LedgerCache::with_ttl(mock.clone(), LONG_TTL);

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
        let cache = Arc::new(LedgerCache::with_ttl(mock.clone(), LONG_TTL));

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
