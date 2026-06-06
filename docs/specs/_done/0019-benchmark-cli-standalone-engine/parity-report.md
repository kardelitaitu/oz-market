# Parity Report — Benchmark CLI and Standalone Engine

| Item | Status | Details |
|------|--------|---------|
| binary scaffold | ✅ **IMPLEMENTED** | `backend/server/src/bin/bench_suite.rs` as `[[bin]] bench_suite` in `Cargo.toml` |
| CLI arguments | ✅ **IMPLEMENTED** | clap `#[derive(Parser)]` handles `--role`, `--target`, `--rate`, `--duration`, `--concurrency`, `--db-url (env DATABASE_URL)`, `--addr`, `--coordinator-addr`, `--workers`, `--base-url`, `--db-max-connections` |
| fixed-rate scheduler | ✅ **IMPLEMENTED** | `scheduler::run_rate_loop()` with interval-based fixed-rate dispatch, coordinated omission correction (latency measured from scheduled tick time, not start), semaphore-based concurrency control, tokio::spawn per operation |
| HDR Histogram | ✅ **IMPLEMENTED** | `hdrhistogram` crate (v7.5, `serialization` feature), `Histogram<u64>` with 1µs–60s range and 3 significant figures, records every operation latency including errors, serialization via `V2Serializer`/`Deserializer` for distributed mode |

## Acceptance Criteria Coverage

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Benchmark CLI target compiled successfully | ✅ | `cargo build --bin bench_suite` succeeds; target registered in `Cargo.toml` `[[bin]]` |
| Clap argument parsing handles role standalone, targets, rate limits, and duration | ✅ | Full CLI parse in `Args` struct — `--role standalone|coordinator|worker`, `--target mock|postgres|cache|wal|sse|http`, `--rate`, `--duration`, `--concurrency` |
| Coordinated omission correction fixed-rate scheduler executes tasks at precise intervals | ✅ | Interval-based loop at `1_000_000 / rate_qps` µs per tick; `schedule_time` recorded before `tokio::spawn`; latency computed as `Instant::now() - schedule_time`; semaphore gates concurrency |
| HDR Histogram library records latency samples locally | ✅ | `Histogram::new_with_bounds(1, 60_000_000, 3)` recorded per operation; returned as `(Histogram<u64>, u64)` tuple with error count |

## Files

- `backend/server/src/bin/bench_suite.rs` — binary entrypoint, CLI, driver wiring
- `backend/server/src/bench/scheduler.rs` — fixed-rate coordinated omission scheduler
- `backend/server/src/bench/mod.rs` — module declaration
