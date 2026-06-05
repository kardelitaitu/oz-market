# Baseline - Benchmark Resource Profiling and CI Gating

## Current State

As of starting Phase 4:
- The system lacks hardware resource monitoring during benchmarks.
- No regression gating exists; tests pass even if average latency grows significantly.
- Benchmarks are not executed as part of the local CI checker loop (`check.ps1`).
