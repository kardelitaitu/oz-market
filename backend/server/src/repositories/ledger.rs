use crate::domain::ledger::{
    CreditAccount, CreditLedgerError, CreditLedgerRepository, CreditTransaction, NewTransaction,
    TransactionType,
};
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::postgres::{PgPool, PgRow};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

fn storage(msg: impl Into<String>) -> CreditLedgerError {
    CreditLedgerError::DatabaseError(msg.into())
}

fn duplicate_key(msg: impl Into<String>) -> CreditLedgerError {
    CreditLedgerError::DuplicateIdempotencyKey(msg.into())
}

fn not_found(id: impl Into<String>) -> CreditLedgerError {
    CreditLedgerError::AgentNotFound(id.into())
}

fn row_to_credit_account(row: &PgRow) -> Result<CreditAccount, CreditLedgerError> {
    let balance: Decimal = row
        .try_get("balance_credits")
        .map_err(|e| storage(format!("parse balance_credits: {e}")))?;
    Ok(CreditAccount {
        agent_id: row
            .try_get("agent_id")
            .map_err(|e| storage(format!("parse agent_id: {e}")))?,
        balance_credits: balance,
        updated_at: row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
            .map(|dt| dt.to_rfc3339())
            .map_err(|e| storage(format!("parse updated_at: {e}")))?,
    })
}

fn row_to_credit_transaction(row: &PgRow) -> Result<CreditTransaction, CreditLedgerError> {
    let tx_type_str: String = row
        .try_get("transaction_type")
        .map_err(|e| storage(format!("parse transaction_type: {e}")))?;
    let tx_type: TransactionType = tx_type_str
        .parse()
        .map_err(|e: String| storage(format!("invalid transaction_type: {tx_type_str}: {e}")))?;
    Ok(CreditTransaction {
        id: row
            .try_get("id")
            .map_err(|e| storage(format!("parse id: {e}")))?,
        agent_id: row
            .try_get("agent_id")
            .map_err(|e| storage(format!("parse agent_id: {e}")))?,
        amount: row
            .try_get("amount")
            .map_err(|e| storage(format!("parse amount: {e}")))?,
        transaction_type: tx_type,
        idempotency_key: row
            .try_get("idempotency_key")
            .map_err(|e| storage(format!("parse idempotency_key: {e}")))?,
        created_at: row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .map(|dt| dt.to_rfc3339())
            .map_err(|e| storage(format!("parse created_at: {e}")))?,
    })
}

#[derive(Debug, Default)]
pub struct InMemoryCreditLedgerRepository {
    balances: RwLock<HashMap<String, CreditAccount>>,
    transactions: RwLock<Vec<CreditTransaction>>,
    idempotency_keys: RwLock<HashMap<String, Uuid>>,
}

impl InMemoryCreditLedgerRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CreditLedgerRepository for InMemoryCreditLedgerRepository {
    async fn get_balance(&self, agent_id: &str) -> Result<CreditAccount, CreditLedgerError> {
        let balances = self.balances.read().map_err(|e| storage(e.to_string()))?;
        balances
            .get(agent_id)
            .cloned()
            .ok_or_else(|| not_found(agent_id))
    }

    async fn apply_transaction(
        &self,
        tx: &NewTransaction,
    ) -> Result<CreditAccount, CreditLedgerError> {
        {
            let keys = self
                .idempotency_keys
                .read()
                .map_err(|e| storage(e.to_string()))?;
            if keys.contains_key(&tx.idempotency_key) {
                return Err(duplicate_key(&tx.idempotency_key));
            }
        }

        let (new_balance, updated_at) = {
            let mut balances = self.balances.write().map_err(|e| storage(e.to_string()))?;
            let account = balances
                .entry(tx.agent_id.clone())
                .or_insert_with(|| CreditAccount {
                    agent_id: tx.agent_id.clone(),
                    balance_credits: Decimal::ZERO,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                });

            let new_bal = account.balance_credits + tx.amount;
            if new_bal < Decimal::ZERO {
                return Err(CreditLedgerError::InsufficientCredits {
                    requested: tx.amount,
                    available: account.balance_credits,
                });
            }

            account.balance_credits = new_bal;
            let ts = chrono::Utc::now().to_rfc3339();
            account.updated_at = ts.clone();
            (new_bal, ts)
        };

        let transaction = CreditTransaction {
            id: tx.id,
            agent_id: tx.agent_id.clone(),
            amount: tx.amount,
            transaction_type: tx.tx_type,
            idempotency_key: tx.idempotency_key.clone(),
            created_at: updated_at.clone(),
        };

        self.transactions
            .write()
            .map_err(|e| storage(e.to_string()))?
            .push(transaction);

        self.idempotency_keys
            .write()
            .map_err(|e| storage(e.to_string()))?
            .insert(tx.idempotency_key.clone(), tx.id);

        Ok(CreditAccount {
            agent_id: tx.agent_id.clone(),
            balance_credits: new_balance,
            updated_at,
        })
    }

    async fn get_transaction_history(
        &self,
        agent_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<CreditTransaction>, CreditLedgerError> {
        let transactions = self
            .transactions
            .read()
            .map_err(|e| storage(e.to_string()))?;
        let filtered: Vec<CreditTransaction> = transactions
            .iter()
            .filter(|t| t.agent_id == agent_id)
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();
        Ok(filtered)
    }
}

#[derive(Debug)]
pub struct PostgresCreditLedgerRepository {
    pool: PgPool,
}

impl PostgresCreditLedgerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CreditLedgerRepository for PostgresCreditLedgerRepository {
    async fn get_balance(&self, agent_id: &str) -> Result<CreditAccount, CreditLedgerError> {
        let row = sqlx::query(
            "SELECT agent_id, balance_credits, updated_at FROM agent_balances WHERE agent_id = $1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(e.to_string()))?
        .ok_or_else(|| not_found(agent_id))?;
        row_to_credit_account(&row)
    }

    async fn apply_transaction(
        &self,
        tx: &NewTransaction,
    ) -> Result<CreditAccount, CreditLedgerError> {
        let mut pg_tx = self
            .pool
            .begin()
            .await
            .map_err(|e| storage(format!("begin transaction: {e}")))?;

        let balance_row = sqlx::query(
            "SELECT agent_id, balance_credits, updated_at FROM agent_balances WHERE agent_id = $1 FOR UPDATE",
        )
        .bind(&tx.agent_id)
        .fetch_optional(pg_tx.as_mut())
        .await
        .map_err(|e| storage(format!("select for update: {e}")))?;

        let (agent_id, current_balance) = match balance_row {
            Some(row) => {
                let agent_id: String = row
                    .try_get("agent_id")
                    .map_err(|e| storage(format!("parse agent_id: {e}")))?;
                let balance: Decimal = row
                    .try_get("balance_credits")
                    .map_err(|e| storage(format!("parse balance: {e}")))?;
                (agent_id, balance)
            }
            None => {
                sqlx::query(
                    "INSERT INTO agent_balances (agent_id, balance_credits) VALUES ($1, 0.0000) ON CONFLICT (agent_id) DO NOTHING",
                )
                .bind(&tx.agent_id)
                .execute(pg_tx.as_mut())
                .await
                .map_err(|e| storage(format!("insert balance row: {e}")))?;

                (tx.agent_id.clone(), Decimal::ZERO)
            }
        };

        let new_balance = current_balance + tx.amount;
        if new_balance < Decimal::ZERO {
            return Err(CreditLedgerError::InsufficientCredits {
                requested: tx.amount,
                available: current_balance,
            });
        }

        sqlx::query(
            "UPDATE agent_balances SET balance_credits = $1, updated_at = NOW() WHERE agent_id = $2",
        )
        .bind(new_balance)
        .bind(&agent_id)
        .execute(pg_tx.as_mut())
        .await
        .map_err(|e| storage(format!("update balance: {e}")))?;

        let result = sqlx::query(
            r#"INSERT INTO credit_transactions (id, agent_id, amount, transaction_type, idempotency_key)
               VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(tx.id)
        .bind(&tx.agent_id)
        .bind(tx.amount)
        .bind(tx.tx_type.as_str())
        .bind(&tx.idempotency_key)
        .execute(pg_tx.as_mut())
        .await;

        if let Err(ref e) = result {
            if let Some(db_err) = e.as_database_error() {
                if let Some(code) = db_err.code() {
                    if code.as_ref() == "23505" {
                        return Err(duplicate_key(&tx.idempotency_key));
                    }
                }
            }
            return Err(storage(format!("insert transaction: {e}")));
        }

        pg_tx
            .commit()
            .await
            .map_err(|e| storage(format!("commit: {e}")))?;

        Ok(CreditAccount {
            agent_id,
            balance_credits: new_balance,
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn get_transaction_history(
        &self,
        agent_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<CreditTransaction>, CreditLedgerError> {
        let rows = sqlx::query(
            r#"SELECT id, agent_id, amount, transaction_type, idempotency_key, created_at
               FROM credit_transactions
               WHERE agent_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(agent_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| storage(e.to_string()))?;

        rows.iter().map(row_to_credit_transaction).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ledger::TransactionType;

    fn make_tx(agent_id: &str, amount: Decimal, tx_type: TransactionType) -> NewTransaction {
        NewTransaction {
            id: Uuid::new_v4(),
            agent_id: agent_id.to_string(),
            amount,
            tx_type,
            idempotency_key: format!("{agent_id}-{}", Uuid::new_v4()),
        }
    }

    #[tokio::test]
    async fn in_memory_deposit_increases_balance() {
        let repo = InMemoryCreditLedgerRepository::new();

        let tx = make_tx("agent-1", Decimal::new(5000, 4), TransactionType::Deposit);
        let _ = repo
            .apply_transaction(&tx)
            .await
            .expect("deposit should succeed");

        let account = repo
            .get_balance("agent-1")
            .await
            .expect("should find agent-1");
        assert_eq!(account.balance_credits, Decimal::new(5000, 4));
    }

    #[tokio::test]
    async fn in_memory_spend_reduces_balance() {
        let repo = InMemoryCreditLedgerRepository::new();

        let deposit = make_tx("agent-2", Decimal::new(10000, 4), TransactionType::Deposit);
        let _ = repo.apply_transaction(&deposit).await.unwrap();

        let spend = make_tx("agent-2", Decimal::new(-3000, 4), TransactionType::Spend);
        let _ = repo.apply_transaction(&spend).await.unwrap();

        let account = repo.get_balance("agent-2").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(7000, 4));
    }

    #[tokio::test]
    async fn in_memory_insufficient_credits_returns_error() {
        let repo = InMemoryCreditLedgerRepository::new();

        let deposit = make_tx("agent-3", Decimal::new(1000, 4), TransactionType::Deposit);
        let _ = repo.apply_transaction(&deposit).await.unwrap();

        let spend = make_tx("agent-3", Decimal::new(-2000, 4), TransactionType::Spend);
        let err = repo.apply_transaction(&spend).await.unwrap_err();

        assert!(
            matches!(&err, CreditLedgerError::InsufficientCredits { .. }),
            "expected InsufficientCredits, got {err}"
        );
    }

    #[tokio::test]
    async fn in_memory_duplicate_idempotency_key_rejected() {
        let repo = InMemoryCreditLedgerRepository::new();

        let tx = NewTransaction {
            id: Uuid::new_v4(),
            agent_id: "agent-4".into(),
            amount: Decimal::new(5000, 4),
            tx_type: TransactionType::Deposit,
            idempotency_key: "dup-key".into(),
        };
        let _ = repo.apply_transaction(&tx).await.unwrap();

        let dup = NewTransaction {
            id: Uuid::new_v4(),
            ..tx
        };
        let err = repo.apply_transaction(&dup).await.unwrap_err();
        assert!(
            matches!(&err, CreditLedgerError::DuplicateIdempotencyKey(k) if k == "dup-key"),
            "expected DuplicateIdempotencyKey, got {err}"
        );
    }

    #[tokio::test]
    async fn in_memory_get_balance_unknown_agent_returns_error() {
        let repo = InMemoryCreditLedgerRepository::new();
        let err = repo.get_balance("unknown").await.unwrap_err();
        assert!(
            matches!(&err, CreditLedgerError::AgentNotFound(id) if id == "unknown"),
            "expected AgentNotFound, got {err}"
        );
    }

    #[tokio::test]
    async fn in_memory_transaction_history_ordered() {
        let repo = InMemoryCreditLedgerRepository::new();

        let tx1 = make_tx("agent-5", Decimal::new(1000, 4), TransactionType::Deposit);
        let tx2 = make_tx("agent-5", Decimal::new(-200, 4), TransactionType::Spend);
        let tx3 = make_tx("agent-5", Decimal::new(500, 4), TransactionType::Refund);

        let _ = repo.apply_transaction(&tx1).await.unwrap();
        let _ = repo.apply_transaction(&tx2).await.unwrap();
        let _ = repo.apply_transaction(&tx3).await.unwrap();

        let history = repo
            .get_transaction_history("agent-5", 10, 0)
            .await
            .unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].amount, Decimal::new(1000, 4));
        assert_eq!(history[1].amount, Decimal::new(-200, 4));
        assert_eq!(history[2].amount, Decimal::new(500, 4));
    }

    #[tokio::test]
    async fn in_memory_spend_exact_balance_allowed() {
        let repo = InMemoryCreditLedgerRepository::new();

        let deposit = make_tx("agent-6", Decimal::new(5000, 4), TransactionType::Deposit);
        let _ = repo.apply_transaction(&deposit).await.unwrap();

        let spend = make_tx("agent-6", Decimal::new(-5000, 4), TransactionType::Spend);
        let _ = repo.apply_transaction(&spend).await.unwrap();

        let account = repo.get_balance("agent-6").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::ZERO);
    }

    #[tokio::test]
    async fn in_memory_multiple_agents_isolated() {
        let repo = InMemoryCreditLedgerRepository::new();

        let tx_a = make_tx("alice", Decimal::new(10000, 4), TransactionType::Deposit);
        let tx_b = make_tx("bob", Decimal::new(5000, 4), TransactionType::Deposit);
        let _ = repo.apply_transaction(&tx_a).await.unwrap();
        let _ = repo.apply_transaction(&tx_b).await.unwrap();

        let alice = repo.get_balance("alice").await.unwrap();
        let bob = repo.get_balance("bob").await.unwrap();
        assert_eq!(alice.balance_credits, Decimal::new(10000, 4));
        assert_eq!(bob.balance_credits, Decimal::new(5000, 4));
    }

    #[tokio::test]
    async fn in_memory_transaction_history_respects_limit_and_offset() {
        let repo = InMemoryCreditLedgerRepository::new();

        for i in 0..5 {
            let tx = make_tx(
                "agent-7",
                Decimal::new(i * 1000, 4),
                TransactionType::Deposit,
            );
            let _ = repo.apply_transaction(&tx).await.unwrap();
        }

        let page = repo.get_transaction_history("agent-7", 2, 1).await.unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].amount, Decimal::new(1000, 4));
        assert_eq!(page[1].amount, Decimal::new(2000, 4));
    }

    #[tokio::test]
    async fn in_memory_adjustment_can_increase_or_decrease() {
        let repo = InMemoryCreditLedgerRepository::new();

        let deposit = make_tx("agent-8", Decimal::new(10000, 4), TransactionType::Deposit);
        let _ = repo.apply_transaction(&deposit).await.unwrap();

        let adj_up = make_tx(
            "agent-8",
            Decimal::new(5000, 4),
            TransactionType::Adjustment,
        );
        let _ = repo.apply_transaction(&adj_up).await.unwrap();

        let adj_down = make_tx(
            "agent-8",
            Decimal::new(-3000, 4),
            TransactionType::Adjustment,
        );
        let _ = repo.apply_transaction(&adj_down).await.unwrap();

        let account = repo.get_balance("agent-8").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(12000, 4));
    }

    #[tokio::test]
    async fn in_memory_refund_reverses_spend() {
        let repo = InMemoryCreditLedgerRepository::new();

        let deposit = make_tx("agent-9", Decimal::new(10000, 4), TransactionType::Deposit);
        let _ = repo.apply_transaction(&deposit).await.unwrap();

        let spend = make_tx("agent-9", Decimal::new(-7000, 4), TransactionType::Spend);
        let _ = repo.apply_transaction(&spend).await.unwrap();

        let refund = make_tx("agent-9", Decimal::new(7000, 4), TransactionType::Refund);
        let _ = repo.apply_transaction(&refund).await.unwrap();

        let account = repo.get_balance("agent-9").await.unwrap();
        assert_eq!(account.balance_credits, Decimal::new(10000, 4));
    }

    #[tokio::test]
    async fn in_memory_empty_history_for_unknown_agent() {
        let repo = InMemoryCreditLedgerRepository::new();
        let history = repo.get_transaction_history("nobody", 10, 0).await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn in_memory_concurrent_spends_no_overdraw() {
        use std::sync::Arc;
        // Spec 0010 validation checklist: "concurrent deposit/spend actions to
        // verify database transaction rollbacks". The in-memory repo's RwLock is
        // the closest analogue we can test without a real Postgres.
        let repo = Arc::new(InMemoryCreditLedgerRepository::new());

        // Seed balance = 100 (10 credits per spend, 50 concurrent attempts).
        let seed = NewTransaction {
            id: Uuid::new_v4(),
            agent_id: "agent-race".into(),
            amount: Decimal::new(1000000, 4), // 100.0000
            tx_type: TransactionType::Deposit,
            idempotency_key: "seed".into(),
        };
        repo.apply_transaction(&seed).await.unwrap();

        let mut handles = Vec::new();
        for i in 0..50 {
            let r = Arc::clone(&repo);
            handles.push(tokio::spawn(async move {
                let tx = NewTransaction {
                    id: Uuid::new_v4(),
                    agent_id: "agent-race".into(),
                    amount: Decimal::new(-100000, 4), // -10.0000
                    tx_type: TransactionType::Spend,
                    idempotency_key: format!("spend-{i}"),
                };
                r.apply_transaction(&tx).await
            }));
        }

        let mut ok = 0;
        let mut insufficient = 0;
        for h in handles {
            match h.await.unwrap() {
                Ok(_) => ok += 1,
                Err(CreditLedgerError::InsufficientCredits { .. }) => insufficient += 1,
                Err(e) => panic!("unexpected error: {e}"),
            }
        }

        assert_eq!(ok, 10, "exactly 10 spends should succeed");
        assert_eq!(insufficient, 40, "remaining 40 should be rejected");

        let final_balance = repo.get_balance("agent-race").await.unwrap();
        assert_eq!(
            final_balance.balance_credits,
            Decimal::ZERO,
            "final balance must be exactly zero — no overdraw, no leftover"
        );
    }
}
