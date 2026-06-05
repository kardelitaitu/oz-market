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

## Drift Notes

Appended 2026-06-05 during the docs-auditor / TODO.md audit. Captures intentional divergences between this plan and the actual implementation, so future readers don't mistake them for bugs.

### `agent_id` is `TEXT PRIMARY KEY`, not a UUID FK to `agents(id)`

The plan originally specified `agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE`. The actual migration (`backend/server/migrations/0014_add_credit_ledger.sql:3-8, 15-23`) uses `agent_id TEXT PRIMARY KEY` on `agent_balances` and `agent_id TEXT NOT NULL` (no FK) on `credit_transactions`. The `agents` table in this codebase is for human-facing user profiles; the credit ledger identifies callers by an opaque credential subject string (e.g. `agent-1`, a JWT `sub` claim, a guest session id), not a UUID foreign key. The `agent_balances` table has a SQL comment (`0014_add_credit_ledger.sql:10-11`) clarifying that `agent_id` "logically references `agent_credentials.subject`" but no database-level FK is enforced.

### `CreditLedgerError::AgentNotFound` carries a `String`, not a `Uuid`

`backend/server/src/domain/ledger.rs:73` declares `AgentNotFound(String)`. All call sites (`backend/server/src/domain/ledger.rs:85, 167, 200`) pass a `String` agent_id. The trait method `CreditLedgerRepository::get_balance` (`backend/server/src/domain/ledger.rs:111`) takes `agent_id: &str`. This matches the schema choice above.

### Why the string-keyed, multi-account design

- **Multi-tenant / guest agents**: anonymous guest agents and federated identity providers both need ledger accounts, but neither has a row in the local `agents` table. A string `subject` accommodates any caller identity (human user UUID, machine agent name, guest session id) without forcing a local profile row.
- **No CASCADE hazard**: a deleted `agents` row should NOT wipe the corresponding ledger (credits are an asset; deletion of a profile is a profile-management concern, not a financial one).
- **Schema simplicity**: a single `TEXT PRIMARY KEY` is portable and avoids a join on every balance read. The `agent_credentials.subject` invariant is enforced at the application layer (in `MarketplaceApp::create_listing`, etc.), not at the database layer.
- **Idempotency-key compatibility**: the `idempotency_key` column is also `VARCHAR(255)`, so a string-typed agent_id fits the same indexing and uniqueness story.
