# Baseline - Credit/Balance DB Schema and Domain Logic

## Current State

As of the start of Phase 3, the codebase has no credit, token ledger, or balance mechanisms:
- There are no tables inside the Postgres database representing credits, balances, or transaction histories.
- The `backend/server/src/domain/` directory has repository traits and structures for listings, negotiations, and contact reveals, but nothing for credits.
- All requests are authenticated and authorized via JWT claims/auth-core, but there is no check or logic to debit or lock credits upon agent queries or listing creation.
