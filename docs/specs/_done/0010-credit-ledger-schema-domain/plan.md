# Plan - Credit/Balance DB Schema and Domain Logic

## Implementation Steps

### 1. Database Migration Setup
* Create `backend/migrations/0014_add_credit_ledger.sql`.
* Define `agent_balances` table with:
  * Primary key `agent_id` referencing the existing `agents(id)` table with `ON DELETE CASCADE`.
  * Column `balance_credits` of type `NUMERIC(20, 4)` starting with default `0.0000`.
  * Check constraint `CHECK (balance_credits >= 0.0000)` to prevent negative credit states at the database layer.
* Define `credit_transactions` table with:
  * Primary key `id` as `UUID`.
  * Foreign key `agent_id` referencing `agents(id)`.
  * Column `amount` of type `NUMERIC(20, 4)` indicating transaction delta.
  * Column `transaction_type` of type `VARCHAR(50)` constrained to `deposit`, `spend`, `refund`, `adjustment`.
  * Unique constraint on `idempotency_key VARCHAR(255)`.
* Define indexes:
  * B-tree index on `credit_transactions(agent_id)` for quick ledger history reads.
  * B-tree index on `credit_transactions(created_at DESC)` for pagination sorting.

### 2. Rust Domain Structs and Repository
* Implement `backend/server/src/domain/ledger.rs` declaring:
  * `CreditAccount` and `CreditTransaction` structs.
  * `TransactionType` enum mapped via serde or diesel/sqlx custom types.
  * `CreditLedgerError` enum encapsulating:
    * `InsufficientCredits { requested: Decimal, available: Decimal }`
    * `DuplicateIdempotencyKey(String)`
    * `AgentNotFound(Uuid)`
    * `DatabaseError(String)`
  * `CreditLedgerRepository` async trait.

### 3. Orchestration and Row Locking in Updates
* The `apply_transaction` method inside the PostgreSQL implementation of the repository must execute:
  1. Start a database transaction.
  2. Execute a row lock read: `SELECT balance_credits FROM agent_balances WHERE agent_id = $1 FOR UPDATE`. If the agent does not exist, initialize a balance row with `0.0000` balance.
  3. Validate balance capacity: if the transaction amount is negative (a spend) and the balance is less than the absolute value of the spend, return `InsufficientCredits`.
  4. Perform the balance update: `UPDATE agent_balances SET balance_credits = balance_credits + $2, updated_at = NOW() WHERE agent_id = $1`.
  5. Log the transaction entry: `INSERT INTO credit_transactions (id, agent_id, amount, transaction_type, idempotency_key, created_at) VALUES ($1, $2, $3, $4, $5, $6)`.
  6. Commit the transaction and return the fresh balance.
  7. Handle unique constraint violations on `idempotency_key` by rolling back and returning `DuplicateIdempotencyKey`.
