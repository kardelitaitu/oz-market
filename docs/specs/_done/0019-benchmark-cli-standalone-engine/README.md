---
id: 0019-benchmark-cli-standalone-engine
title: Benchmark CLI and Standalone Engine
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Benchmark CLI and Standalone Engine

Status: `active`
Implementer: `agent`

## Summary

This specification defines the CLI framework and the standalone execution engine for the backend benchmark suite. It establishes the command-line argument structure, the local task scheduler with coordinated omission correction, and HDR Histogram logging.

## Scope

### In Scope
- Configuring the `bench-suite` binary target in `backend/server/Cargo.toml`.
- Parsing arguments (`--role`, `--target`, `--rate`, `--duration`, `--concurrency`).
- Implementing a time-aligned, fixed-rate worker scheduler for coordinated omission correction.
- Recording latency distributions using the `hdrhistogram` crate.

### Out of Scope
- gRPC coordinator-worker distributed clustering protocols (deferred to Spec 0021).
- Target driver details (deferred to Spec 0020).

## Proposed Direction
1. Scaffolding:
   - Introduce `backend/server/src/bin/bench_suite.rs` as the binary.
   - Use `clap` to validate inputs.
2. Local Scheduler:
   - Set up an interval loop that dispatches actions to workers at precise intervals (e.g., based on target QPS) rather than waiting for previous requests to complete.
