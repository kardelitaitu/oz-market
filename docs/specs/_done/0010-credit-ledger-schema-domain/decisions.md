# Decisions - Credit/Balance DB Schema and Domain Logic

## Architecture Decisions

### 1. High-Precision Decimals for Balances
- **Decision**: We will use `numeric(20, 4)` in Postgres and `rust_decimal::Decimal` in Rust.
- **Rationale**: To allow micro-credits and sub-credit operations without floating point inaccuracies or round-off bugs.

### 2. Idempotency Key Constraint on Transactions
- **Decision**: The `credit_transactions` table will have a unique constraint on `idempotency_key`.
- **Rationale**: Any retry by client or task runner must not double-spend or double-deposit credits if a network error occurred after commit.

### 3. Transactional Integrity (ACID)
- **Decision**: Balance updates and transaction records must be committed within the same database transaction.
- **Rationale**: Ensures the audit log is always mathematically consistent with the agent's current balance.
