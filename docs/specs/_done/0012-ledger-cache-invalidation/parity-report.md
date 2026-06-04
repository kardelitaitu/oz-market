# Parity Report - Cache Invalidation and Admin Interventions

| Item | Status | Details |
|------|--------|---------|
| TTL Expiry | ✅ **DONE** | `CachedEntry` with `Instant` timestamp; `get_balance` evicts on expiry (ZERO_TTL test) |
| Invalidation Hook | ✅ **DONE** | `invalidate` + `invalidate_all` in `LedgerCache`; admin endpoint calls `apply_transaction` (write-through) |
| HTTP Controller | ✅ **DONE** | `POST /internal/v1/sellers/{seller_id}/credits` with admin role guard; 6 actix-web tests |
