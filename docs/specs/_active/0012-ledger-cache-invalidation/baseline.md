# Baseline - Cache Invalidation and Admin Interventions

## Current State

As of the start of Phase 3:
- The `openapi.yaml` spec defines various administrative routes under `/api/admin/` (such as setting trust levels and recalculating ratings), but has no endpoint for credits or balances.
- The `LedgerCache` (defined in Spec 0011) has no expiration policy and holds cached items indefinitely, which would result in stale data if external processes modified the DB directly.
