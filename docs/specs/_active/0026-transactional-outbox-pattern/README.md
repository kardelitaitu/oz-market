---
id: 0026-transactional-outbox-pattern
title: Transactional Outbox Pattern
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Transactional Outbox Pattern

Status: `active`
Implementer: `agent`

## Summary

This specification defines the implementation of a Transactional Outbox Pattern for domain events. It guarantees at-least-once event delivery by committing audit/outbox events to the database in the same transaction as business updates and using a background polling process to publish them.

## Scope

### In Scope
- Designing the database schema for the outbox table.
- Writing queries to commit outbox events inside Postgres transactions.
- Creating the background `OutboxPublisher` task.
- Marking events as published and sweeping completed records.

### Out of Scope
- Integrating external message brokers (event delivery is local broadcast or direct HTTP push).

## Proposed Direction
1. Outbox Table:
   - Create `outbox_events` table: `id` (UUID), `event_type` (TEXT), `payload` (JSONB), `status` (TEXT: pending/published), `created_at` (TIMESTAMP).
2. Background Worker:
   - Polling loop query: `SELECT * FROM outbox_events WHERE status = 'pending' ORDER BY created_at LIMIT 100 FOR UPDATE SKIP LOCKED`.
   - Dispatch events and update status to `published`.
