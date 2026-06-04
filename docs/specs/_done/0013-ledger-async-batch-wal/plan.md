# Plan - Write-Ahead Log (WAL) and Async Batch Committer

## Implementation Steps

### 1. WAL File Format & Write Performance
* Choose JSON Lines (`.jsonl`) serialization for the WAL log for simple reading and debugging, appended line-by-line.
* Each line contains a JSON object:
  `{"transaction_id":"...","agent_id":"...","amount":"...","idempotency_key":"...","tx_type":"..."}`
* Call `sync_all()` on the file handle for every append to force physical flush to disk storage.

### 2. Recovery State Machine on Startup
* Before binding Actix/TCP servers:
  1. Check if `ledger.wal` exists. If not, exit recovery and start server.
  2. If it exists, open the file and parse lines line-by-line.
  3. Ignore/skip any corrupt or incomplete JSON lines (partially written files due to crash during flush).
  4. For each valid transaction record:
     * Check if the transaction exists in the database by executing:
       `SELECT EXISTS(SELECT 1 FROM credit_transactions WHERE idempotency_key = $1)`
     * If it does not exist, run a DB transaction to apply the balance shift and insert the audit log record.
  5. Once all lines are parsed and reconciled, truncate the `ledger.wal` file to `0` bytes (or delete it).

### 3. Asynchronous Batch Loop Execution
* A background task loops continuously:
  1. Maintains a buffer of queued transactions.
  2. Wakes up upon receiving an entry or when the batch timer reaches `100ms`.
  3. Consolidates balance changes: if the same agent has multiple adjustments in the batch, combine them to a single delta to reduce SQL load.
  4. Execute a bulk SQL statement:
     ```sql
     -- Bulk upsert query consolidating all balance adjustments in the batch
     INSERT INTO agent_balances (agent_id, balance_credits, updated_at)
     VALUES ($1, $2, NOW())
     ON CONFLICT (agent_id) DO UPDATE
     SET balance_credits = agent_balances.balance_credits + EXCLUDED.balance_credits, updated_at = NOW();
     ```
  5. Bulk insert transaction logs into `credit_transactions`.
  6. Confirm batch completion, wake up awaiting tasks, and flush/truncate the local WAL file.

### 4. Caching & Batch Metrics
* Track performance indicators via atomic variables exposed on a metrics route `/v1/metrics`:
  * `ledger_cache_hit_total` (counter)
  * `ledger_cache_miss_total` (counter)
  * `ledger_batch_lag_milliseconds` (gauge showing duration from queue push to DB commit)
  * `ledger_batch_size` (gauge showing number of entries in last batch)
