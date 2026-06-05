# Plan - Cache Invalidation and Admin Interventions

## Implementation Steps

### 1. Endpoint Contract Addition
* Update `docs/specs/openapi.yaml` to include the administrative balance adjustment route:
  `POST /internal/v1/sellers/{seller_id}/credits` (see `openapi.yaml:670`; the plan originally specified the un-namespaced `POST /v1/admin/sellers/{id}/credits`).
* Define payload validation structures:
  * Check that `amount` is a string representation of a positive, finite decimal.
  * Check that `adjustment` matches one of the expected enum keys (`deposit`, `spend`, `refund`, `adjustment`).
  * Enforce that `idempotency_key` is a non-empty string.

### 2. Authorization Security Rule
* Wire the endpoint to require a JWT with claims containing `"role": "admin"`.
* If a request lacks valid claims or the role is not `admin`, return `403 Forbidden` with a JSON body indicating missing scopes.

### 3. HTTP Request & Response Payload Layout

#### Request Schema
```json
{
  "adjustment": "deposit",
  "amount": "150.0000",
  "idempotency_key": "adm-ref-0012-abc"
}
```

#### Successful Response (200 OK)
```json
{
  "agent_id": "8f3a38d7-d954-4a49-9df2-8e100cdb4c9b",
  "balance_credits": "150.0000",
  "idempotency_key": "adm-ref-0012-abc",
  "updated_at": "2026-06-04T15:30:00Z"
}
```

> **Drift note (2026-06-05)**: the handler implementing this route is `adjust_credits` in `backend/server/src/http/actix_handlers.rs:1526` (the plan originally said `backend/server/src/http/handlers.rs::admin_adjust_credits`). It is registered at `actix_handlers.rs:1496` under the `/internal/v1` scope. The response body, status code, and idempotency contract described in this §3 are unchanged.

#### Error Response: Insufficient Balance (400 Bad Request)
```json
{
  "error": "insufficient_credits",
  "message": "Insufficient credits: requested 20.0000, available 10.0000"
}
```

#### Error Response: Duplicate Idempotency (409 Conflict)
```json
{
  "error": "duplicate_idempotency_key",
  "message": "Transaction with idempotency key 'adm-ref-0012-abc' already exists"
}
```
