# Plan - Benchmark CLI and Standalone Engine

## Implementation Steps

1. **Target Scaffolding**:
   - Register `[[bin]]` with name `bench-suite` in `backend/server/Cargo.toml`.
   - Setup `backend/server/src/bin/bench_suite.rs` with `main` entry point.

2. **CLI Argument Parsing**:
   - Use `clap` to declare options for targets, role standalone, concurrency, rate limits, and duration.

3. **Coordinated Omission Scheduler**:
   - Implement an interval-based task scheduler using `tokio::time::interval` or spin-locking high-resolution counters.
   - Dispatch runner tasks at strict intervals computed as `1.0 / target_qps`.

4. **HDR Histogram Recording**:
   - Import `hdrhistogram` library.
   - Instantiate a thread-safe histogram buffer and record local latencies in microseconds.
