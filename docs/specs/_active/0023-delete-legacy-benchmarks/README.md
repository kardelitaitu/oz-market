---
id: 0023-delete-legacy-benchmarks
title: Delete Legacy Benchmarks
status: active
owner: backend-team
implementer: agent
priority: P3
---

# Delete Legacy Benchmarks

Status: `active`
Implementer: `agent`

## Summary

This specification defines the cleanup requirements to remove all legacy, fragmented, and redundant benchmark scripts, files, and targets from the repository now that a unified benchmark suite is planned.

## Scope

### In Scope
- Deleting legacy benchmark source files under `backend/server/src/bin/` (`bench_concurrent.rs`, `http_bench.rs`, `pg_search_bench.rs`, `phase5_bench.rs`).
- Deleting benchmark runner scripts under `backend/scripts/` and `backend/server/scripts/` (`bench-http.ps1`, `run-phase5-bench-local.ps1`, `run-phase5-bench.ps1`).
- Deleting Criterion benchmark files under `backend/server/benches/` (`search_bench.rs`).
- Cleaning up associated configurations inside `backend/server/Cargo.toml`.

### Out of Scope
- Removing database population utilities (like `populate_db.rs`) that are required for seeding.

## Proposed Direction
1. Source Deletion:
   - Remove files from filesystem.
2. Cargo Configuration:
   - Audit `backend/server/Cargo.toml` and remove any `[[bin]]` or `[[bench]]` targets referencing deleted files.
