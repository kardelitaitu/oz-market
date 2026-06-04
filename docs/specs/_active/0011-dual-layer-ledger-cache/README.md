---
id: 0011-dual-layer-ledger-cache
title: Dual-Layer Ledger Trait and Synchronous Cache
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Dual-Layer Ledger Trait and Synchronous Cache

Status: `active`
Implementer: `agent`

## Summary

This specification outlines the design and implementation of a thread-safe `DashMap`-backed caching layer that wraps the PostgreSQL ledger database, implementing a synchronous write-through model.

## Scope

### In Scope
- Designing the `LedgerCache` structure utilizing `DashMap` for concurrent in-memory balance reads.
- Implementing write-through mechanics where all spends and deposits are committed immediately to the Postgres DB while updating the in-memory balance.
- Ensuring thread safety of balance read checks.

### Out of Scope
- Implementing background thread pooling or dirty-write flushing.
- Multi-instance horizontal replication (which would require Redis). DashMap is sufficient for single-process operations.

## Proposed Direction
1. Service interface:
   - Create `backend/server/src/services/ledger_cache.rs`.
   - Implement read methods that lookup the agent's balance in `DashMap`. If missing, query Postgres, populate cache, and return.
   - Implement mutations (spend, deposit) that execute DB transaction first, then update the local `DashMap` entry.
