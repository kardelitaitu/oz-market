# CLI README

Common commands for day-to-day development and testing.

## From project root

### Verify code

```powershell
.\check.ps1
```

### Format code

```powershell
cd backend
cargo fmt --all
```

### Lint

```powershell
cd backend
cargo clippy --workspace
```

### Run tests

```powershell
cd backend
cargo test --lib
```

## Backend development

### Start the HTTP server

```powershell
cd backend
cargo run --package marketplace-server
```

### Seed the database once

```powershell
cargo run --manifest-path backend/Cargo.toml -p marketplace-server --bin populate_db
```

### Run the real HTTP benchmark

#### Full Benchmark Suite (Recommended)
```powershell
.\backend\scripts\bench-http.ps1 -Ops 1000 -ConcurrencyLevels "1,10,50,100,250,500,1000" -SeedDatabase
```
- **What it does**: Starts Postgres, seeds database, starts Actix server, runs comprehensive benchmarks
- **Best for**: Complete end-to-end performance testing

#### Quick Sequential Benchmark
```powershell
cd backend
cargo run --release --bin http_bench -- "http://127.0.0.1:3000" 5000
```
- **What it does**: Sends requests one-after-another (sequential) to measure basic throughput
- **Best for**: Quick performance checks, CI/CD pipelines
- **Measures**: Single-threaded performance with cache warming effects

#### Concurrent Load Testing
```powershell
cd backend
cargo run --release --bin bench_concurrent -- "http://127.0.0.1:3000" 1000 "1,10,50,100"
```
- **What it does**: Tests multiple concurrent users hitting the server simultaneously
- **Best for**: Load testing, scalability validation
- **Measures**: How system performs under real-world concurrent load

### Benchmark Concepts

#### Sequential vs Concurrent
- **Sequential**: Requests sent one at a time (like a single user clicking)
- **Concurrent**: Multiple requests sent simultaneously (like many users at once)
- **Why both matter**: Real apps serve many users concurrently

#### Cold vs Warm Cache
- **Cold Cache**: First request after server start (database/cache not primed)
- **Warm Cache**: Subsequent requests (data cached in memory)
- **Performance difference**: Often 100x+ faster after cache warms up

#### Key Metrics
- **ops/s**: Operations per second (higher = better)
- **p50/p95**: Response time percentiles (lower = better)
- **success_rate**: Percentage of requests that succeed

### Run the local Postgres dev flow

```powershell
.\backend\server\scripts\run-local-postgres-dev.ps1
```

## Backend database checks

### Benchmark against local Postgres

```powershell
.\backend\server\scripts\run-phase5-bench-local.ps1
```

### Run Postgres integration tests

```powershell
.\backend\server\scripts\run-postgres-tests-local.ps1
```

## Notes

- Use `check.ps1` before commits.
- Prefer the HTTP benchmark when measuring real server performance.
- Seed once, benchmark many.
- The real benchmark is the Actix server + HTTP path, not the direct app/repo benchmark.
- Use lower concurrency first, then sweep higher levels to see saturation.
- Current baseline (Phase 1 complete): release build with Actix + Moka cache
  - Sequential search: 6,346 ops/s
  - Concurrent search (100 threads): 46,407 ops/s (9.3x Phase 1 target)
  - Get listing: 52,356 ops/s
  - All benchmarks achieve 100% success rate with sub-millisecond p50 latency
