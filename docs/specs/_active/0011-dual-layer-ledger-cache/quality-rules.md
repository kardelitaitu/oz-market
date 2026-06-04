# Quality Rules - Dual-Layer Ledger Trait and Synchronous Cache

- **No Cache Modification on DB Error**: The in-memory cache must not be updated if the SQL write fails or rolls back.
- **Cache Eviction Cap**: While cache size limits are not explicitly enforced for early builds, the cache should implement a TTL or simple eviction limit to prevent unbound memory leak on long-running instances.
- **Mockability**: The `LedgerCache` should be structured to accept a mocked database repository during unit tests.
