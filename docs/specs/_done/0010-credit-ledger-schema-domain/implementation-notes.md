# Implementation Notes - Credit/Balance DB Schema and Domain Logic

## Schema Definitions

```sql
-- Migration: 0014_add_credit_ledger.sql

CREATE TABLE agent_balances (
    agent_id UUID PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
    balance_credits NUMERIC(20, 4) NOT NULL DEFAULT 0.0000,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_positive_balance CHECK (balance_credits >= 0.0000)
);

CREATE TABLE credit_transactions (
    id UUID PRIMARY KEY,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    amount NUMERIC(20, 4) NOT NULL,
    transaction_type VARCHAR(50) NOT NULL,
    idempotency_key VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_tx_type CHECK (transaction_type IN ('deposit', 'spend', 'refund', 'adjustment'))
);

CREATE INDEX idx_credit_transactions_agent_id ON credit_transactions(agent_id);
CREATE INDEX idx_credit_transactions_created_at ON credit_transactions(created_at DESC);
```

## Domain Error Types

```rust
use rust_decimal::Decimal;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum CreditLedgerError {
    #[error("Agent {0} not found")]
    AgentNotFound(Uuid),

    #[error("Insufficient credits: requested {requested}, available {available}")]
    InsufficientCredits {
        requested: Decimal,
        available: Decimal,
    },

    #[error("Transaction with idempotency key '{0}' already exists")]
    DuplicateIdempotencyKey(String),

    #[error("Database error occurred: {0}")]
    DatabaseError(String),
}
```

## Domain Repository Trait

```rust
use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NewTransaction {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub amount: Decimal,
    pub tx_type: TransactionType,
    pub idempotency_key: String,
}

#[async_trait]
pub trait CreditLedgerRepository: Send + Sync {
    async fn get_balance(&self, agent_id: &Uuid) -> Result<Decimal, CreditLedgerError>;
    async fn apply_transaction(&self, tx: &NewTransaction) -> Result<CreditAccount, CreditLedgerError>;
    async fn get_transaction_history(&self, agent_id: &Uuid, limit: usize, offset: usize) -> Result<Vec<CreditTransaction>, CreditLedgerError>;
}
```
