# Validation Checklist - Credit/Balance DB Schema and Domain Logic

This checklist is used to confirm the completion of Spec 0010:

- [ ] Database migration is placed under `backend/migrations/` and correctly formats balance/transaction tables.
- [ ] Schema applies successfully inside Postgres during test migrations.
- [ ] Rust structures `CreditAccount` and `CreditTransaction` compile without errors in `backend/server`.
- [ ] `CreditLedgerRepository` trait compiles cleanly.
- [ ] Unit tests are written covering concurrent deposit/spend actions to verify database transaction rollbacks and idempotency constraints.
- [ ] Integration tests verify that duplicate idempotency keys return a structured conflict error instead of crashing.
