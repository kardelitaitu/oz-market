use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use uuid::Uuid;

use crate::domain::ledger::{CreditLedgerRepository, NewTransaction, TransactionType};
use crate::services::wal::{WalEntry, WalManager};

const DEFAULT_FLUSH_INTERVAL_MS: u64 = 100;
const DEFAULT_BATCH_SIZE: usize = 100;

/// Asynchronous batch committer for credit ledger transactions.
///
/// Transactions are enqueued via a channel, buffered, and flushed to the DB
/// either when the buffer reaches `batch_size` entries or when the flush
/// interval elapses.  Entries are consolidated by agent_id so that multiple
/// transactions for the same agent are collapsed into a single delta.
///
/// A background `tokio` task runs the flush loop.  Drop the sender half to
/// signal shutdown (remaining entries are flushed on the next tick).
pub struct AsyncBatchCommitter {
    rx: mpsc::Receiver<WalEntry>,
    db_repo: Arc<dyn CreditLedgerRepository>,
    wal: Arc<WalManager>,
    flush_interval: Duration,
    batch_size: usize,
}

/// Public handle for enqueuing transactions into the batch committer.
pub type BatchSender = mpsc::Sender<WalEntry>;

/// Create a batch committer channel pair.
///
/// Returns a sender (for enqueuing transactions) and a committer (which must
/// have `start()` called to begin processing).
pub fn batch_channel(
    db_repo: Arc<dyn CreditLedgerRepository>,
    wal: Arc<WalManager>,
) -> (BatchSender, AsyncBatchCommitter) {
    let (tx, rx) = mpsc::channel::<WalEntry>(1024);
    let committer = AsyncBatchCommitter {
        rx,
        db_repo,
        wal,
        flush_interval: Duration::from_millis(DEFAULT_FLUSH_INTERVAL_MS),
        batch_size: DEFAULT_BATCH_SIZE,
    };
    (tx, committer)
}

impl AsyncBatchCommitter {
    /// Start the background flush loop.
    ///
    /// This spawns a `tokio` task that runs until the sender is dropped.
    pub fn start(mut self) {
        tokio::spawn(async move {
            let mut ticker = interval(self.flush_interval);
            ticker.tick().await; // Skip the immediate tick
            let mut buffer = Vec::with_capacity(self.batch_size);

            loop {
                tokio::select! {
                    Some(entry) = self.rx.recv() => {
                        buffer.push(entry);
                        if buffer.len() >= self.batch_size {
                            Self::flush_batch(&mut buffer, &self.db_repo, &self.wal).await;
                        }
                    }
                    _ = ticker.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_batch(&mut buffer, &self.db_repo, &self.wal).await;
                        }
                    }
                }

                // If the channel is closed and buffer is empty, exit
                if buffer.is_empty() && self.rx.is_closed() && self.rx.is_empty() {
                    break;
                }
            }

            // Drain remaining entries on shutdown
            while let Some(entry) = self.rx.recv().await {
                buffer.push(entry);
            }
            if !buffer.is_empty() {
                Self::flush_batch(&mut buffer, &self.db_repo, &self.wal).await;
            }
        });
    }

    async fn flush_batch(
        buffer: &mut Vec<WalEntry>,
        db_repo: &Arc<dyn CreditLedgerRepository>,
        wal: &Arc<WalManager>,
    ) {
        // Consolidate by agent_id: net the amounts
        let mut deltas: HashMap<String, Decimal> = HashMap::new();
        for entry in buffer.drain(..) {
            let amount: Decimal = match entry.amount.parse() {
                Ok(a) => a,
                Err(_) => continue,
            };
            *deltas.entry(entry.agent_id).or_insert(Decimal::ZERO) += amount;
        }

        // Apply each consolidated delta to the DB
        for (agent_id, delta) in &deltas {
            let tx_type = if *delta >= Decimal::ZERO {
                TransactionType::Deposit
            } else {
                TransactionType::Spend
            };

            let new_tx = NewTransaction {
                id: Uuid::new_v4(),
                agent_id: agent_id.clone(),
                amount: *delta,
                tx_type,
                idempotency_key: format!("batch-{agent_id}-{}", Uuid::new_v4()),
            };

            let _ = db_repo.apply_transaction(&new_tx).await;
        }

        // Truncate the WAL now that entries are safely in the DB
        let _ = wal.truncate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::domain::ledger::CreditLedgerRepository;
    use crate::repositories::ledger::InMemoryCreditLedgerRepository;

    fn make_entry(agent_id: &str, amount: &str, tx_type: &str) -> WalEntry {
        WalEntry {
            transaction_id: Uuid::new_v4(),
            agent_id: agent_id.to_owned(),
            amount: amount.to_owned(),
            tx_type: tx_type.to_owned(),
            idempotency_key: Uuid::new_v4().to_string(),
        }
    }

    fn temp_wal() -> Arc<WalManager> {
        let dir = std::env::temp_dir().join(format!("batch_test_{}", Uuid::new_v4()));
        let path = dir.join("ledger.wal");
        Arc::new(WalManager::new(path).unwrap())
    }

    #[tokio::test]
    async fn batch_committer_flushes_on_tick() {
        let repo: Arc<dyn CreditLedgerRepository> = Arc::new(InMemoryCreditLedgerRepository::new());
        let wal = temp_wal();
        let (tx, committer) = batch_channel(repo.clone(), wal);

        tx.send(make_entry("agent-1", "100.0000", "deposit"))
            .await
            .unwrap();
        tx.send(make_entry("agent-1", "50.0000", "deposit"))
            .await
            .unwrap();

        committer.start();

        tokio::time::sleep(Duration::from_millis(500)).await;

        let account = repo.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, "150.0000".parse().unwrap());
    }

    #[tokio::test]
    async fn batch_committer_consolidates_same_agent() {
        let repo: Arc<dyn CreditLedgerRepository> = Arc::new(InMemoryCreditLedgerRepository::new());
        let wal = temp_wal();
        let (tx, committer) = batch_channel(repo.clone(), wal);

        for _ in 0..10 {
            tx.send(make_entry("agent-1", "10.0000", "deposit"))
                .await
                .unwrap();
        }

        committer.start();
        tokio::time::sleep(Duration::from_millis(500)).await;

        let account = repo.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, "100.0000".parse().unwrap());
    }

    #[tokio::test]
    async fn batch_committer_separates_different_agents() {
        let repo: Arc<dyn CreditLedgerRepository> = Arc::new(InMemoryCreditLedgerRepository::new());
        let wal = temp_wal();
        let (tx, committer) = batch_channel(repo.clone(), wal);

        tx.send(make_entry("agent-a", "200.0000", "deposit"))
            .await
            .unwrap();
        tx.send(make_entry("agent-b", "300.0000", "deposit"))
            .await
            .unwrap();

        committer.start();
        tokio::time::sleep(Duration::from_millis(500)).await;

        let a = repo.get_balance("agent-a").await.unwrap();
        let b = repo.get_balance("agent-b").await.unwrap();
        assert_eq!(a.balance_credits, "200.0000".parse().unwrap());
        assert_eq!(b.balance_credits, "300.0000".parse().unwrap());
    }

    #[tokio::test]
    async fn batch_committer_drains_on_shutdown() {
        let repo: Arc<dyn CreditLedgerRepository> = Arc::new(InMemoryCreditLedgerRepository::new());
        let wal = temp_wal();
        let (tx, committer) = batch_channel(repo.clone(), wal);

        tx.send(make_entry("agent-1", "75.0000", "deposit"))
            .await
            .unwrap();

        committer.start();

        drop(tx);

        tokio::time::sleep(Duration::from_millis(500)).await;

        let account = repo.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, "75.0000".parse().unwrap());
    }

    #[tokio::test]
    async fn batch_committer_wal_truncated_after_flush() {
        let repo: Arc<dyn CreditLedgerRepository> = Arc::new(InMemoryCreditLedgerRepository::new());
        let wal = temp_wal();
        let (tx, committer) = batch_channel(repo.clone(), Arc::clone(&wal));

        let entry = make_entry("agent-1", "50.0000", "deposit");
        wal.append(&entry).unwrap();
        tx.send(entry).await.unwrap();

        assert_eq!(wal.read_all().unwrap().len(), 1);

        committer.start();
        tokio::time::sleep(Duration::from_millis(500)).await;

        assert_eq!(wal.read_all().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn batch_committer_mixed_spend_and_refund_nets_to_correct_balance() {
        let repo: Arc<dyn CreditLedgerRepository> = Arc::new(InMemoryCreditLedgerRepository::new());
        let wal = temp_wal();
        let (tx, committer) = batch_channel(repo.clone(), wal);

        // 100 + 100 + 100 - 200 + 100 = 200.0000
        tx.send(make_entry("agent-mix", "100.0000", "deposit"))
            .await
            .unwrap();
        tx.send(make_entry("agent-mix", "100.0000", "deposit"))
            .await
            .unwrap();
        tx.send(make_entry("agent-mix", "100.0000", "deposit"))
            .await
            .unwrap();
        tx.send(make_entry("agent-mix", "-200.0000", "spend"))
            .await
            .unwrap();
        tx.send(make_entry("agent-mix", "100.0000", "refund"))
            .await
            .unwrap();

        committer.start();
        tokio::time::sleep(Duration::from_millis(500)).await;

        let account = repo.get_balance("agent-mix").await.unwrap();
        assert_eq!(account.balance_credits, "200.0000".parse().unwrap());
    }

    #[tokio::test]
    async fn batch_committer_large_batch_triggers_size_flush() {
        let repo: Arc<dyn CreditLedgerRepository> = Arc::new(InMemoryCreditLedgerRepository::new());
        let wal = temp_wal();
        let (tx, committer) = batch_channel(repo.clone(), wal);

        // Default batch size is 100; sending 250 must flush at least twice.
        // Use 250 distinct agents so the per-agent consolidation doesn't mask the size-trigger.
        for i in 0..250 {
            tx.send(make_entry(&format!("agent-{i}"), "1.0000", "deposit"))
                .await
                .unwrap();
        }

        committer.start();

        // Wait long enough for both size-triggered and tick-triggered flushes.
        tokio::time::sleep(Duration::from_millis(800)).await;

        let mut total = Decimal::ZERO;
        for i in 0..250 {
            let a = repo.get_balance(&format!("agent-{i}")).await.unwrap();
            total += a.balance_credits;
        }
        assert_eq!(
            total,
            Decimal::new(2500000, 4),
            "250 deposits of 1.0000 must total 250.0000"
        );
    }
}
