---
id: 0020-benchmark-component-drivers
title: Benchmark Component Drivers
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Benchmark Component Drivers

Status: `active`
Implementer: `agent`

## Summary

This specification governs the implementation of the `BenchmarkDriver` trait and its concrete targets (Postgres, Ledger Cache, WAL, SSE channels, HTTP router).

## Scope

### In Scope
- Defining the `BenchmarkDriver` trait interface.
- Creating the `CacheDriver` querying the DashMap ledger cache.
- Creating the `PostgresDriver` invoking database reads/writes.
- Creating the `WalDriver` executing sequential logs writes with `fsync` file syncs.
- Creating the `SseDriver` and `HttpDriver` testing event broadcast and API routing.

### Out of Scope
- Command line arguments parsing (managed by Spec 0019).
- gRPC synchronization protocol (deferred to Spec 0021).

## Proposed Direction
1. Driver Trait:
   - Implement an async trait with `setup`, `run_operation`, and `teardown` hooks.
2. Drivers:
   - Wire each driver with the relevant backend service/connection pool.
   - Clean up files/states during `teardown`.
