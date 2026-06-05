---
id: 0024-distributed-ledger-cache-redis
title: Distributed Ledger Cache Synchronization
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Distributed Ledger Cache Synchronization

Status: `active`
Implementer: `agent`

## Summary

This specification governs the implementation of a distributed caching layer for the ledger using Redis. It replaces single-process in-memory caching with a shared distributed cache, utilizing Redis Pub/Sub for cross-instance invalidations.

## Scope

### In Scope
- Integrating a Redis client driver with the `LedgerCache` service.
- Implementing write-through balance updates to PostgreSQL and Redis.
- Defining a Redis Pub/Sub channel for cache invalidation notifications.
- Handling connection drop fallbacks to query Postgres directly.

### Out of Scope
- Storing transactions histories in Redis (Postgres remains the historical source of truth).

## Proposed Direction
1. Redis Cache integration:
   - Use `redis` crate with async connections.
   - Cache keys match format: `ledger:balance:{agent_id}`.
2. Invalidation Protocol:
   - On balance mutation, publish the modified `agent_id` to `ledger:invalidation` pub/sub channel.
   - Instances listening to the channel evict the corresponding key from their local caches.
