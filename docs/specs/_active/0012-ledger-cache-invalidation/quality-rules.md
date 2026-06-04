# Quality Rules - Cache Invalidation and Admin Interventions

- **Strict Access Controls**: The admin credits endpoint must be protected by the `Admin` permission guard, rejecting any non-admin JWT claims with a `403 Forbidden` response.
- **Idempotent Admin Edits**: All admin adjustments must supply and record an idempotency key to prevent repeating batch awards/penalties on connection retries.
- **Validation Constraints**: Negative credit values must be rejected during JSON parsing or validation (amount must be positive; adjustment type determines add/subtract behavior).
