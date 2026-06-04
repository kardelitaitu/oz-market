# Baseline - Dual-Layer Ledger Trait and Synchronous Cache

## Current State

As of the start of Phase 3:
- Database transaction models and repository structures exist (defined in Spec 0010) but require direct queries to Postgres for every check.
- The `SlidingWindowRateLimiter` serves as a precedent for utilizing thread-safe in-memory maps (`DashMap`) to manage rapid operations.
- No cache exists for balance operations, resulting in database round-trips for every billing/credit check.
