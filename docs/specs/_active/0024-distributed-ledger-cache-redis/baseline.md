# Baseline - Distributed Ledger Cache Synchronization

## Current State

As of starting Phase 4:
- The `LedgerCache` uses a local single-process `DashMap` which does not coordinate with other server instances.
- No Redis client dependency or caching configurations exist in the backend.
- Cache invalidation only evicts keys locally in the current process.
