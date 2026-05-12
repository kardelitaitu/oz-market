---
id: 0004-http-benchmark-stability
title: HTTP Benchmark Stability and Reproducibility
status: active
owner: backend-team
implementer: opencode
priority: P1
area:
  - backend
  - api
  - testing
files:
  code:
    - backend/server/src/bin/bench_concurrent.rs
    - backend/server/src/http/runtime.rs
    - backend/server/src/http/actix_handlers.rs
  docs:
    - README.md
    - docs/server/README.md
    - docs/testing/benchmarks/http-bench-baseline-2026-05-12.md
acceptance:
  - benchmark runs are reproducible with explicit claims mode
  - benchmark output reports ops/s with explicit 429 and other failure counts
  - benchmark docs define canonical command set for public, rotating, and fixed modes
  - benchmark artifacts include dated baseline evidence and root-cause notes
  - full ./check.ps1 passes after benchmark-related updates
non_goals:
  - changing product search semantics
  - disabling rate limiter for production paths
  - introducing transport-specific business logic divergence
risks:
  - misleading throughput claims if claims mode is omitted
  - accidental comparison between non-equivalent benchmark runs
  - noisy baseline updates without dated artifact discipline
---

# HTTP Benchmark Stability and Reproducibility

Status: `active`

Owner: `backend-team`
Implementer: `opencode`

## Summary

Standardize benchmark execution and reporting so throughput comparisons are reliable, repeatable, and auditable across days.

## Scope

### In Scope

- explicit claims mode in benchmark commands
- explicit failure metrics (`429`, `other_failures`) in summaries
- dated benchmark artifacts with a baseline report
- contract/runtime parity checks for benchmarked routes

### Out of Scope

- changing business rules behind rate limiting
- replacing benchmark toolchain with a new framework
- broad performance optimization beyond benchmark correctness

## Current Baseline

The repository now has benchmark artifacts for `public`, `rotating`, and `fixed` claims modes dated `2026-05-12`.

Recent investigation showed that fixed-sub tests hit search limiter thresholds and can under-report sustainable throughput if compared directly with rotating/public runs.

## Target Outcome

A single canonical benchmark process that keeps comparisons apples-to-apples and captures enough metadata for audit and diagnosis.
