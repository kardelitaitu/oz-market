# Baseline - Benchmark Component Drivers

## Current State

As of starting Phase 4:
- The backend has no pluggable `BenchmarkDriver` trait.
- Benchmark drivers for Postgres, LedgerCache, WAL, SSE events, and the HTTP layer do not exist.
- Each temporary benchmark script re-implements connection setups, task loops, and cleanup logic.
