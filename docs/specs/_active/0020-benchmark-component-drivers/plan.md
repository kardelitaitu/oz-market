# Plan - Benchmark Component Drivers

## Implementation Steps

1. **Driver Trait Declaration**:
   - Define the async `BenchmarkDriver` trait in `backend/server/src/bin/bench_suite.rs` or driver module.

2. **Ledger Cache Driver**:
   - Implement `CacheDriver` accessing `LedgerCache` instance.
   - Run operation executes cache reads and writes under simulated transaction parameters.

3. **Postgres Driver**:
   - Implement `PostgresDriver` holding a `PgPool` instance.
   - Run operation performs parameterized SELECT queries and INSERT operations.

4. **WAL Driver**:
   - Implement `WalDriver` writing serialized transaction records to a temp file, calling `.sync_all()` on each operation.

5. **SSE and HTTP Drivers**:
   - Implement `SseDriver` utilizing reqwest client streams.
   - Implement `HttpDriver` using actix handler dispatch loops.
