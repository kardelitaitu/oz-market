# Parity Report - Credit/Balance DB Schema and Domain Logic

| Item | Status | Details |
|------|--------|---------|
| DB Migration | ✅ **DONE** | `backend/server/migrations/0014_add_credit_ledger.sql` — creates `agent_balances` and `credit_transactions` tables with CHECK constraints, indexes, comments |
| Domain Models | ✅ **DONE** | `backend/server/src/domain/ledger.rs` — `CreditAccount`, `CreditTransaction`, `NewTransaction` structs, `TransactionType` enum (impl FromStr), `CreditLedgerError` enum with Display |
| Repositories | ✅ **DONE** | `backend/server/src/repositories/ledger.rs` — `CreditLedgerRepository` trait, `InMemoryCreditLedgerRepository` (auto-creates balances, idempotency guard), `PostgresCreditLedgerRepository` (SELECT FOR UPDATE, transactional) |
