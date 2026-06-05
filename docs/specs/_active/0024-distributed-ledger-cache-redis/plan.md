# Plan - Distributed Ledger Cache Synchronization

## Implementation Steps

1. **Add Redis Dependency**:
   - Add `redis` with features `tokio-comp` and `connection-manager` to Cargo.toml.

2. **Distributed Cache Client**:
   - Create `backend/server/src/services/ledger_cache_distributed.rs`.
   - Implement `DistributedLedgerCache` wrapping a Redis connection manager and a Postgres database repository.

3. **Pub/Sub Listener**:
   - Spawn a background task on server startup that subscribes to `ledger:invalidation`.
   - Evict target keys from local secondary memory when invalidation events are received.

4. **Fallback Handling**:
   - Implement error boundaries: if Redis query or write fails, log an error and query Postgres directly to preserve service availability.
