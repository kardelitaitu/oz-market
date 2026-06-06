# Plan - Delete Legacy Benchmarks

## Implementation Steps

1. **Delete Files**:
   - Delete `backend/scripts/bench-http.ps1`.
   - Delete `backend/server/benches/search_bench.rs`.
   - Delete `backend/server/scripts/run-phase5-bench-local.ps1`.
   - Delete `backend/server/scripts/run-phase5-bench.ps1`.
   - Delete `backend/server/src/bin/bench_concurrent.rs`.
   - Delete `backend/server/src/bin/http_bench.rs`.
   - Delete `backend/server/src/bin/pg_search_bench.rs`.
   - Delete `backend/server/src/bin/phase5_bench.rs`.

2. **Clean Cargo.toml**:
   - Audit `backend/server/Cargo.toml`.
   - Remove any `[[bin]]` sections declaring:
     - `bench_concurrent`
     - `http_bench`
     - `pg_search_bench`
     - `phase5_bench`
   - Remove `[[bench]]` section declaring `search_bench`.
   - Remove `criterion` from dev-dependencies if no other benches use it.
