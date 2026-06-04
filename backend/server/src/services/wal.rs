use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::ledger::{CreditLedgerRepository, NewTransaction, TransactionType};

const DEFAULT_WAL_PATH: &str = "./data/ledger.wal";

/// A single entry in the Write-Ahead Log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    pub transaction_id: Uuid,
    pub agent_id: String,
    pub amount: String,
    pub tx_type: String,
    pub idempotency_key: String,
}

#[derive(Debug)]
pub enum WalError {
    Io(std::io::Error),
    Parse(String),
    Ledger(crate::domain::ledger::CreditLedgerError),
}

impl std::fmt::Display for WalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalError::Io(e) => write!(f, "WAL I/O error: {e}"),
            WalError::Parse(msg) => write!(f, "WAL parse error: {msg}"),
            WalError::Ledger(e) => write!(f, "WAL ledger error: {e}"),
        }
    }
}

impl std::error::Error for WalError {}

impl From<std::io::Error> for WalError {
    fn from(e: std::io::Error) -> Self {
        WalError::Io(e)
    }
}

/// Manages an append-only Write-Ahead Log (WAL) on local disk.
///
/// Entries are serialized as JSON Lines (`.jsonl`). Every append is followed
/// by `sync_all()` to guarantee durability on power loss.
pub struct WalManager {
    file: Mutex<File>,
    path: PathBuf,
}

impl WalManager {
    /// Open (or create) the WAL file at the given path.
    pub fn new(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        Ok(Self {
            file: Mutex::new(file),
            path,
        })
    }

    /// Open the default WAL path from `LEDGER_WAL_PATH` env var or `./data/ledger.wal`.
    pub fn from_env() -> std::io::Result<Self> {
        let path = std::env::var("LEDGER_WAL_PATH").unwrap_or_else(|_| DEFAULT_WAL_PATH.to_owned());
        Self::new(path)
    }

    /// Append one entry and `sync_all()` to disk immediately.
    pub fn append(&self, entry: &WalEntry) -> std::io::Result<()> {
        let line = serde_json::to_string(entry)?;
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(())
    }

    /// Read all entries currently in the WAL file.
    ///
    /// Corrupt or unparseable lines are silently skipped.
    pub fn read_all(&self) -> std::io::Result<Vec<WalEntry>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => break,
            };
            let trimmed = line.trim().to_owned();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str(&trimmed) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// Truncate the WAL file to zero bytes after a successful DB flush.
    ///
    /// On Windows, `set_len` is not permitted on append-only handles, so we
    /// replace the inner file with a fresh write-truncated handle.
    pub fn truncate(&self) -> std::io::Result<()> {
        let new_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&self.path)?;
        let mut file = self.file.lock().unwrap();
        *file = new_file;
        file.sync_all()?;
        Ok(())
    }

    /// Recover uncommitted transactions from the WAL on boot.
    ///
    /// Reads all entries, checks each against the DB by idempotency key, and
    /// applies any missing transactions. Truncates the WAL when complete.
    pub async fn recover(&self, db_repo: &dyn CreditLedgerRepository) -> Result<(), WalError> {
        let entries = self.read_all()?;
        if entries.is_empty() {
            return Ok(());
        }

        for entry in &entries {
            let tx_type = match entry.tx_type.as_str() {
                "deposit" => TransactionType::Deposit,
                "spend" => TransactionType::Spend,
                "refund" => TransactionType::Refund,
                "adjustment" => TransactionType::Adjustment,
                _ => continue,
            };

            let amount: Decimal = match entry.amount.parse() {
                Ok(a) => a,
                Err(_) => continue,
            };

            let new_tx = NewTransaction {
                id: entry.transaction_id,
                agent_id: entry.agent_id.clone(),
                amount,
                tx_type,
                idempotency_key: entry.idempotency_key.clone(),
            };

            // Attempt apply — duplicate idempotency keys are silently ignored
            let _ = db_repo.apply_transaction(&new_tx).await;
        }

        self.truncate()?;
        Ok(())
    }

    /// Return the path of the WAL file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

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

    fn temp_wal() -> (WalManager, PathBuf) {
        let dir = std::env::temp_dir().join(format!("wal_test_{}", Uuid::new_v4()));
        let path = dir.join("ledger.wal");
        let wal = WalManager::new(&path).unwrap();
        (wal, path)
    }

    #[test]
    fn append_and_read_roundtrip() {
        let (wal, _dir) = temp_wal();
        let entry = make_entry("agent-1", "100.0000", "deposit");

        wal.append(&entry).unwrap();
        let entries = wal.read_all().unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].agent_id, "agent-1");
        assert_eq!(entries[0].amount, "100.0000");
        assert_eq!(entries[0].tx_type, "deposit");
    }

    #[test]
    fn append_multiple_entries() {
        let (wal, _dir) = temp_wal();

        wal.append(&make_entry("a1", "10", "deposit")).unwrap();
        wal.append(&make_entry("a2", "20", "spend")).unwrap();
        wal.append(&make_entry("a1", "5", "refund")).unwrap();

        let entries = wal.read_all().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].agent_id, "a1");
        assert_eq!(entries[1].agent_id, "a2");
        assert_eq!(entries[2].agent_id, "a1");
    }

    #[test]
    fn truncate_clears_file() {
        let (wal, _dir) = temp_wal();

        wal.append(&make_entry("agent-1", "50", "deposit")).unwrap();
        assert_eq!(wal.read_all().unwrap().len(), 1);

        wal.truncate().unwrap();
        assert_eq!(wal.read_all().unwrap().len(), 0);
    }

    #[test]
    fn read_all_skips_corrupt_lines() {
        let (wal, path) = temp_wal();
        use std::io::Write;

        // Write a valid line followed by garbage
        let valid = serde_json::to_string(&make_entry("a1", "10", "deposit")).unwrap();
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "{valid}").unwrap();
        writeln!(f, "this is not json").unwrap();
        writeln!(f, "{{broken").unwrap();
        drop(f);

        let entries = wal.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].agent_id, "a1");
    }

    #[tokio::test]
    async fn recover_applies_missing_transactions() {
        let (wal, _dir) = temp_wal();
        let repo = Arc::new(InMemoryCreditLedgerRepository::new());

        let entry = make_entry("agent-1", "250.0000", "deposit");
        wal.append(&entry).unwrap();

        // Pre-apply the transaction (simulates it already being in DB)
        // recover should handle idempotency — it will get a DuplicateIdempotencyKey error
        // but that's silently ignored by the recover loop
        let new_tx = NewTransaction {
            id: entry.transaction_id,
            agent_id: entry.agent_id.clone(),
            amount: "250.0000".parse().unwrap(),
            tx_type: TransactionType::Deposit,
            idempotency_key: entry.idempotency_key.clone(),
        };

        // This should succeed
        repo.apply_transaction(&new_tx).await.unwrap();

        // Recover — will hit duplicate idempotency key but should continue
        wal.recover(&*repo).await.unwrap();

        // WAL should be truncated
        assert_eq!(wal.read_all().unwrap().len(), 0);

        // Balance should still be 250 (not doubled)
        let account = repo.get_balance("agent-1").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(2500000, 4));
    }

    #[tokio::test]
    async fn recover_creates_balance_for_unknown_agent() {
        let (wal, _dir) = temp_wal();
        let repo = Arc::new(InMemoryCreditLedgerRepository::new());

        wal.append(&make_entry("new-agent", "500.0000", "deposit"))
            .unwrap();

        wal.recover(&*repo).await.unwrap();

        let account = repo.get_balance("new-agent").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(5000000, 4));
    }

    #[tokio::test]
    async fn recover_handles_empty_wal() {
        let (wal, _dir) = temp_wal();
        let repo = Arc::new(InMemoryCreditLedgerRepository::new());
        wal.recover(&*repo).await.unwrap();
        // No panic = success
    }

    #[tokio::test]
    async fn recover_skips_entries_with_unknown_tx_type() {
        let (wal, _dir) = temp_wal();
        let repo = Arc::new(InMemoryCreditLedgerRepository::new());

        wal.append(&make_entry("agent-valid", "100.0000", "deposit"))
            .unwrap();
        wal.append(&make_entry("agent-bogus", "50.0000", "transfer"))
            .unwrap();
        wal.append(&make_entry("agent-valid", "25.0000", "deposit"))
            .unwrap();

        wal.recover(&*repo).await.unwrap();

        let valid = repo.get_balance("agent-valid").await.unwrap();
        assert_eq!(
            valid.balance_credits,
            Decimal::new(1250000, 4),
            "two valid deposits totaling 125.0000"
        );
        assert!(
            repo.get_balance("agent-bogus").await.is_err(),
            "unknown tx_type must not create a balance row"
        );
    }

    #[tokio::test]
    async fn recover_skips_entries_with_unparseable_amount() {
        let (wal, _dir) = temp_wal();
        let repo = Arc::new(InMemoryCreditLedgerRepository::new());

        wal.append(&make_entry("agent-good", "10.0000", "deposit"))
            .unwrap();
        wal.append(&make_entry("agent-bad", "not-a-decimal", "deposit"))
            .unwrap();

        wal.recover(&*repo).await.unwrap();

        let good = repo.get_balance("agent-good").await.unwrap();
        assert_eq!(good.balance_credits, Decimal::new(100000, 4));
        assert!(repo.get_balance("agent-bad").await.is_err());
    }

    #[tokio::test]
    async fn recover_spend_overdraw_is_silently_swallowed() {
        let (wal, _dir) = temp_wal();
        let repo = Arc::new(InMemoryCreditLedgerRepository::new());

        // No prior balance; spend will be rejected by the repo.
        wal.append(&make_entry("agent-broke", "-100.0000", "spend"))
            .unwrap();
        wal.append(&make_entry("agent-broke", "50.0000", "deposit"))
            .unwrap();

        // Recovery must not propagate the InsufficientCredits error.
        wal.recover(&*repo).await.expect("recover must not fail");

        // WAL is truncated regardless.
        assert_eq!(wal.read_all().unwrap().len(), 0);

        // The valid deposit still applied.
        let bal = repo.get_balance("agent-broke").await.unwrap();
        assert_eq!(bal.balance_credits, Decimal::new(500000, 4));
    }

    #[tokio::test]
    async fn recover_does_not_double_apply_when_db_already_has_transaction() {
        let (wal, _dir) = temp_wal();
        let repo = Arc::new(InMemoryCreditLedgerRepository::new());

        let entry = make_entry("agent-once", "500.0000", "deposit");

        // Apply via the repo directly first (simulating a successful prior commit).
        let pre_tx = NewTransaction {
            id: entry.transaction_id,
            agent_id: entry.agent_id.clone(),
            amount: "500.0000".parse().unwrap(),
            tx_type: TransactionType::Deposit,
            idempotency_key: entry.idempotency_key.clone(),
        };
        repo.apply_transaction(&pre_tx).await.unwrap();

        // Then append + recover — must NOT double-apply.
        wal.append(&entry).unwrap();
        wal.recover(&*repo).await.unwrap();

        let bal = repo.get_balance("agent-once").await.unwrap();
        assert_eq!(
            bal.balance_credits,
            Decimal::new(5000000, 4),
            "balance must remain 500.0000 (single application via idempotency)"
        );
    }
}
