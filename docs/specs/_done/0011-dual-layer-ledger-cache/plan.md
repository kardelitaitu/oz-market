# Plan - Dual-Layer Ledger Trait and Synchronous Cache

## Implementation Steps

### 1. In-Memory Struct Design
* Create `backend/server/src/services/ledger_cache.rs` exposing the `LedgerCache` struct.
* Utilize `dashmap::DashMap<String, CachedEntry>` to store one entry per agent. The plan originally specified `DashMap<Uuid, Decimal>` (current balance only); the actual struct stores the full `CreditAccount` so the read path can return the account verbatim, plus an `inserted_at: Instant` for TTL-based eviction. `CachedEntry` is private; `LedgerCache` exposes `get_balance`, `apply_transaction`, `get_transaction_history`, `invalidate`, `invalidate_all`.
* Ensure all cache interactions are non-blocking where possible, avoiding nesting locks to prevent deadlocks under parallel loads. TTL is configurable via `LEDGER_CACHE_TTL_SECS` (default 300s); expired entries are evicted on read.

### 2. Read Cache Miss Resolution
* When checking balances via `get_balance(agent_id)`:
  1. Perform a shard lookup on the `DashMap`.
  2. On cache hit, return the balance instantly (no database call).
  3. On cache miss, execute a DB query: `db_repo.get_balance(agent_id)`.
  4. Acquire a write lock on the `DashMap` key entry and insert the fetched balance.
  5. Return the fresh balance.

### 3. Synchronous Write-Through Logic
* When executing updates (`spend` or `deposit`):
  1. Call `db_repo.apply_transaction(tx)` first to execute the SQL lock, constraint check, and database commit.
  2. If the database transaction succeeds:
     * Immediately write the returned updated balance to the `DashMap` using `.insert()`.
     * Return `Ok(new_balance)`.
  3. If the database transaction fails (e.g. constraint violation, insufficient credits, db timeout):
     * Do **NOT** modify or update the cache.
     * Return the original `CreditLedgerError` bubble.
     * Optionally invalidate the key using `.remove()` if there is any suspicion of sync drift.
