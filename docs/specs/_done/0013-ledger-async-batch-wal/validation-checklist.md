# Validation Checklist - Write-Ahead Log (WAL) and Async Batch Committer

This checklist is used to confirm the completion of Spec 0013:

- [x] `WalManager` correctly serializes transaction records and flushes them to disk synchronously (`append_and_read_roundtrip`, `append_multiple_entries`, `truncate_clears_file`).
- [x] Async batching task consolidates multiple transactions and processes them concurrently under a single DB connection (`batch_committer_consolidates_same_agent`, `batch_committer_separates_different_agents`).
- [x] Application boot checks for uncommitted `ledger.wal` entries and performs full state reconciliation before starting HTTP listener (`recover` called in `async_run`; `recover_applies_missing_transactions`, `recover_creates_balance_for_unknown_agent` tests).
- [ ] Metrics for `cache_hit`, `cache_miss`, and `batch_lag` update correctly in real-time. *(Deferred)*
- [ ] Performance benchmarks demonstrate throughput improvement. *(Deferred)*
