# Validation Checklist - Cache Invalidation and Admin Interventions

This checklist is used to confirm the completion of Spec 0012:

- [ ] Configuration variable `LEDGER_CACHE_TTL_SECS` is defined and read correctly on startup.
- [ ] Ledger cache drops elements that exceed the configured time-to-live threshold.
- [ ] Admin route `POST /v1/admin/sellers/{id}/credits` is defined and secured under admin auth permissions.
- [ ] Modifying balances through the admin controller applies to the PostgreSQL database and immediately executes cache invalidation.
- [ ] Verification tests ensure that a subsequent balance query pulls the new adjusted value from PostgreSQL.
