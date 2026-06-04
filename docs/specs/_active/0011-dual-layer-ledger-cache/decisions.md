# Decisions - Dual-Layer Ledger Trait and Synchronous Cache

## Architecture Decisions

### 1. In-Memory Cache Backed by DashMap
- **Decision**: We will use `dashmap::DashMap` for internal balance caching.
- **Rationale**: DashMap provides fine-grained shard locking, ensuring high-concurrency throughput under heavy parallel read demands.

### 2. Synchronous Write-Through Strategy
- **Decision**: The cache acts as write-through. Any mutation goes to Postgres first, and updates the cache only after a successful DB commit.
- **Rationale**: Mitigates risk of cache drift or database transaction aborts causing false updates to the client.
