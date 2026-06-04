---
id: 0010-credit-ledger-schema-domain
title: Credit/Balance DB Schema and Domain Logic
status: done
owner: backend-team
implementer: agent
priority: P2
---

# Credit/Balance DB Schema and Domain Logic

Status: `done`
Implementer: `agent`

## Summary

This specification outlines the data model and domain requirements to introduce a credit/balance tracking ledger inside the backend application.

## Scope

### In Scope
- Designing DB migrations for `agent_balances` and `credit_transactions` tables.
- Defining Rust domain models: `CreditAccount`, `CreditTransaction`, and transaction types (`deposit`, `spend`, `refund`, `adjustment`).
- Creating `CreditLedgerRepository` trait to provide DB access abstraction.

### Out of Scope
- Creating the cache layer or async committer.
- Charging credits for actual API queries (this spec only implements the storage/logic baseline).

## Proposed Direction
1. Schema:
   - `agent_balances` tracks `agent_id` (PK/FK to agents table), `balance_credits` (decimal or bigint), `updated_at`.
   - `credit_transactions` acts as an audit log tracking `id`, `agent_id`, `amount`, `transaction_type`, `idempotency_key`, `created_at`.
2. Domain logic:
   - Create `backend/server/src/domain/ledger.rs`.
   - Implement `CreditLedgerRepository` trait.
