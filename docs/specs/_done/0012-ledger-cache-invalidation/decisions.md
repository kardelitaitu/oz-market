# Decisions - Cache Invalidation and Admin Interventions

## Architecture Decisions

### 1. Proactive Eviction on Admin Modification
- **Decision**: The admin HTTP controller will perform a database write and call `invalidate()` to evict the value rather than trying to calculate the next cache state manually.
- **Rationale**: Keeps controller code simple and guarantees the cache fetches the exact, constraint-validated value on the next hit.

### 2. Time-To-Live (TTL) Fallback
- **Decision**: A default TTL of 5 minutes (300 seconds) will be applied to cache reads, configurable via the environment variable `LEDGER_CACHE_TTL_SECS`.
- **Rationale**: Prevents indefinite memory consumption and ensures that any out-of-process modifications to PostgreSQL are naturally reconciled.
