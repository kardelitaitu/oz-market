# Baseline - Delete Legacy Benchmarks

## Current State

As of starting Phase 4:
- The codebase contains multiple fragmented and outdated benchmark scripts and binaries (e.g. `http_bench.rs`, `bench_concurrent.rs`, etc.) scattered across bins and script directories.
- `Cargo.toml` contains target configurations for these deleted files.
- Criterion is declared as a dependency for legacy bench setups which may no longer be required.
