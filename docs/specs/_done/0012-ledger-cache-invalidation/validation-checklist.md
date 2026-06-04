# Validation Checklist - Cache Invalidation and Admin Interventions

This checklist is used to confirm the completion of Spec 0012:

- [x] Configuration variable `LEDGER_CACHE_TTL_SECS` is defined and read correctly on startup (default 300s in `LedgerCache::new`).
- [x] Ledger cache drops elements that exceed the configured time-to-live threshold (`get_balance_evicts_expired_entry` test).
- [x] Admin route `POST /internal/v1/sellers/{seller_id}/credits` is defined and secured under admin auth permissions (6 actix-web tests).
- [x] Modifying balances through the admin controller applies to the PostgreSQL database and immediately executes cache invalidation (`apply_transaction` write-through).
- [x] Verification tests ensure that a subsequent balance query pulls the new adjusted value from PostgreSQL (`admin_credits_spend_deducts_and_returns_new_balance`).
