---
id: 0013-ledger-async-batch-wal
title: Write-Ahead Log (WAL) and Async Batch Committer
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Write-Ahead Log (WAL) and Async Batch Committer

Status: `active`
Implementer: `agent`

## Summary

This specification describes the architecture for moving credit transaction logging and commits to an asynchronous batching model. It details the Write-Ahead Log (WAL) mechanism needed on local disk to ensure zero credit loss in the event of a server crash or process termination.

## Scope

### In Scope
- Designing a local, append-only WAL manager in Rust.
- Creating an async batch committer task that aggregates pending updates and commits them to PostgreSQL in batches every `N` milliseconds.
- Recovering uncommitted transactions from the WAL file on application boot.
- Collecting cache performance and lag metrics (`cache_hit`, `cache_miss`, `batch_lag`).

### Out of Scope
- Replicating WAL logs over a network to replica servers.

## Proposed Direction
1. WAL Log Format:
   - Simple JSON lines or binary encoding appended to `ledger.wal`.
   - Each entry contains `id`, `agent_id`, `amount`, `tx_type`, `idempotency_key`, `created_at`.
2. Recovery on Boot:
   - On server startup, read `ledger.wal`, identify transactions not yet present/completed in the Postgres database, execute them, and truncate the WAL file.
3. Async Batch Task:
   - A background loop that wakes up every 100ms, drains queued mutations, performs a single batch update to PostgreSQL, and flags the corresponding WAL entries as cleared/flushed.
