# Implementation Notes - Write-Ahead Log (WAL) and Async Batch Committer

## Recovery File Parsing Logic

```rust
use std::fs::File;
use std::io::{BufRead, BufReader};

impl WalManager {
    pub async fn run_recovery(&self, db_repo: &impl CreditLedgerRepository) -> std::io::Result<()> {
        let file = match File::open(&self.file_path) {
            Ok(f) => f,
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };

        let reader = BufReader::new(file);
        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => {
                    eprintln!("WARNING: Corrupt line encountered in WAL. Stopping parse.");
                    break; // stop parsing on read corruption
                }
            };

            // Attempt deserialization, skip line if corrupt
            let entry: WalEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => {
                    eprintln!("WARNING: JSON deserialization failed for WAL line. Skipping.");
                    continue;
                }
            };

            // Reconcile transaction with PostgreSQL
            let idempotency_key = entry.idempotency_key.clone();
            match db_repo.get_balance(&entry.agent_id).await {
                Ok(_) => {
                    // Check if transaction log already exists in the database
                    // If missing, apply the balance update
                    let tx_type = match entry.tx_type.as_str() {
                        "deposit" => TransactionType::Deposit,
                        "spend" => TransactionType::Spend,
                        "refund" => TransactionType::Refund,
                        "adjustment" => TransactionType::Adjustment,
                        _ => continue,
                    };

                    let new_tx = NewTransaction {
                        id: entry.transaction_id,
                        agent_id: entry.agent_id,
                        amount: entry.amount,
                        tx_type,
                        idempotency_key,
                    };

                    // We execute this directly on DB, bypassing the cache
                    let _ = db_repo.apply_transaction(&new_tx).await;
                }
                Err(err) => {
                    eprintln!("Recovery failed to check agent balance: {:?}", err);
                }
            }
        }

        // Clean up WAL file once recovery completes
        self.truncate()
    }
}
```

## Batch Task Background Worker Skeleton

```rust
use tokio::sync::mpsc::Receiver;
use tokio::time::{interval, Duration};

pub struct AsyncBatchCommitter {
    rx: Receiver<WalEntry>,
    db_repo: Arc<dyn CreditLedgerRepository>,
    wal: Arc<WalManager>,
}

impl AsyncBatchCommitter {
    pub fn start(self) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100));
            let mut buffer = Vec::with_capacity(100);

            loop {
                tokio::select! {
                    Some(entry) = self.rx.recv() => {
                        buffer.push(entry);
                        if buffer.len() >= 100 {
                            Self::flush_batch(&mut buffer, &self.db_repo, &self.wal).await;
                        }
                    }
                    _ = interval.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_batch(&mut buffer, &self.db_repo, &self.wal).await;
                        }
                    }
                }
            }
        });
    }

    async fn flush_batch(
        buffer: &mut Vec<WalEntry>,
        db_repo: &Arc<dyn CreditLedgerRepository>,
        wal: &Arc<WalManager>,
    ) {
        // Consolidated agent deltas
        // Execute dynamic bulk upsert SQL query
        // Clean/truncate the WAL file after database success
        buffer.clear();
    }
}
```
