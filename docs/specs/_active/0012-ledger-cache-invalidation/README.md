---
id: 0012-ledger-cache-invalidation
title: Cache Invalidation and Admin Interventions
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Cache Invalidation and Admin Interventions

Status: `active`
Implementer: `agent`

## Summary

This specification outlines the cache invalidation policy, TTL behavior, and HTTP endpoints required for administrative manual credit adjustments and cache clearing.

## Scope

### In Scope
- Designing time-to-live (TTL) expiration logic for the in-memory cache entries.
- Creating an admin endpoint `POST /v1/admin/sellers/{id}/credits` to add/subtract credits or set absolute credit amounts.
- Providing an invalidation interface `invalidate(agent_id: &Uuid)` to clear specific cache entries.

### Out of Scope
- Adding public non-admin endpoints to purchase credits (handled via placeholder billing system, out of scope for now).

## Proposed Direction
1. TTL Behavior:
   - Cache values will be stored along with a timestamp. If `now - timestamp > TTL`, the entry is treated as a cache miss and re-queried from PostgreSQL.
2. Invalidation:
   - Provide `pub fn invalidate(&self, agent_id: &Uuid)` to drop matching keys from the `DashMap`.
   - Wire the admin route handler to call this invalidation function immediately after executing DB updates.
3. HTTP Endpoint:
   - Handler `admin_adjust_credits` in `backend/server/src/http/handlers.rs`.
