# Baseline - Transactional Outbox Pattern

## Current State

As of starting Phase 4:
- The system writes outbox events directly to a Postgres table (`outbox_events`) but lacks a structured transactional commit boundary or background publisher.
- Event publishing happens in-process without transaction synchronization, leading to potential lost events if a network drop or crash occurs after a DB commit.
