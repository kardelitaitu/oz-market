---
id: 0003-negotiation-offer-history
title: Negotiation Offer History and Finalization
status: completed
owner: backend-team
implementer: opencode
priority: P2
area:
  - backend
  - api
  - mcp
  - mobile
files:
  code:
    - backend/server/src/app.rs
    - backend/server/src/http/runtime.rs
    - backend/server/src/repositories/negotiations.rs
    - backend/server/src/repositories/reservations.rs
    - backend/crates/api-contract/src/negotiation.rs
  docs:
    - docs/specs/openapi.yaml
    - docs/01-whitepaper/09-concurrency-and-reservations.md
    - docs/01-whitepaper/10-api-contract.md
    - docs/01-whitepaper/14-state-machines.md
    - docs/01-whitepaper/21-auth-scopes-and-claims.md
acceptance:
  - negotiation responses include richer offer history
  - offer submission updates the negotiation row atomically
  - accept and reject are explicit API actions
  - reservation remains the finalization gate
  - HTTP, MCP, and mobile share the same payload shapes and idempotency behavior
  - current negotiation state names remain unchanged
non_goals:
  - payment settlement
  - cancel semantics unless later approved
  - state renaming
  - replacing the reservation lease model
risks:
  - extra schema and service work
  - more endpoint surface
  - migration work if offer history is modeled too narrowly
---

# Negotiation Offer History and Finalization

Status: `completed`

Owner: `backend-team`
Implementer: `opencode`

## Summary

Define negotiation as a stateful workflow with richer offer history, negotiation-row-led offer updates, explicit seller accept/reject actions, and reservation as the finalization gate.

## Scope

### In Scope

- richer offer history in the negotiation API
- negotiation-row-led offer tracking
- explicit `accept` and `reject` actions
- reservation-gated finalization
- shared behavior across HTTP, MCP, and mobile

### Out of Scope

- rename of the current negotiation state names
- payment processing
- full cancellation semantics unless the product needs them
- reservation model replacement

## Current Baseline

The current contract already exposes:

- `POST /negotiations`
- `GET /negotiations/{id}`
- `POST /negotiations/{id}/offers`
- `POST /negotiations/{id}/accept`
- `POST /negotiations/{id}/reject`
- `POST /negotiations/{id}/request-contact-reveal`
- `POST /contact-reveals/{id}/approve`

The current runtime already implements:

- reservation leases in the backend runtime
- idempotency on the main write paths
- reservation checks before contact reveal

Current baseline status:

- negotiation repository write paths for submit/accept/reject are implemented
- offer history persistence is in place on the negotiation row

## Proposed Contract Direction

### Offer History

Keep the current latest-offer fields and add an append-only history list to the negotiation response.

Suggested response shape:

```json
{
  "negotiation_id": "neg_123",
  "listing_id": "lst_123",
  "buyer_agent_id": "buyer-agent-1",
  "status": "reserved",
  "offer_currency": "USD",
  "latest_offer_amount": 440,
  "offer_history": [
    {
      "offer_currency": "USD",
      "offer_amount": 430,
      "idempotency_key": "offer-001",
      "submitted_by": "buyer-agent-1",
      "submitted_at": "2026-05-11T00:00:00Z"
    },
    {
      "offer_currency": "USD",
      "offer_amount": 440,
      "idempotency_key": "offer-002",
      "submitted_by": "seller-account-1",
      "submitted_at": "2026-05-11T00:05:00Z"
    }
  ],
  "reservation_lease_id": "lease_123",
  "final_offer_amount": 440,
  "version": 3,
  "updated_at": "2026-05-11T00:05:00Z"
}
```

History entry shape:

- `entry_id`
- `entry_type`
- `offer_currency`
- `offer_amount`
- `actor_subject`
- `actor_role`
- `idempotency_key`
- `resulting_status`
- `created_at`

Allowed `entry_type` values:

- `offer`
- `accept`
- `reject`

Rules:

- the history list is append-only
- every state-changing negotiation write appends one entry
- `accept` and `reject` are direct API actions and also history entries
- `cancel` remains out of scope for this spec
- history entries must preserve ordering by `created_at`

### Offer Writes

- `POST /negotiations/{id}/offers` appends a new offer record
- `submit_offer` should be negotiation-row-led
- idempotency remains required on replay-sensitive offer writes

### Finalization Actions

Add explicit actions for:

- `accept`
- `reject`

Keep `cancel` out of the initial spec unless the product flow truly needs it.

### State Names

Keep the current state names unchanged for the next spec:

- `open`
- `countered`
- `near_close`
- `reserved`
- `contact_requested`
- `contact_revealed`
- `closed`
- `cancelled`

## State Rules

- offers may evolve the negotiation state, but the negotiation row stays the source of truth for offer history
- reservation remains the final gate for reveal and finalization
- acceptance may create or confirm the reservation lease
- reject ends the negotiation without exposing private contact data
- contact reveal still requires a valid matching reservation

## Decisions

| Decision | Value | Reason |
| --- | --- | --- |
| Offer history | API exposes richer history | Clients need visible negotiation history, not audit-only data |
| Offer write model | Negotiation-row-led | Keeps offer state and history together |
| Finalization endpoints | `accept` and `reject` | Clear user action mapping |
| Cancel | deferred | Avoid unnecessary endpoint surface until a real flow needs it |
| State names | unchanged | Prevents churn before the next spec is approved |
| Reservation | final gate | Preserves safety and double-sell protection |

## Plan

### Step 1: Contract shape

- define `offer_history` fields
- keep current `NegotiationResponse` fields intact
- treat `offer_history` as append-only negotiation events

### Step 2: Service shape

- move offer history handling into the negotiation row
- keep reservation as a separate finalization gate
- keep idempotency on `submit_offer`, `accept`, and `reject`

### Step 3: Endpoint shape

- keep `POST /negotiations/{id}/offers`
- keep explicit `accept` and `reject` endpoints
- leave `cancel` out unless the product proves it is needed

### Step 4: Shared behavior

- keep HTTP, MCP, and mobile payloads aligned
- keep the same authz and reservation checks on every surface
- keep state names unchanged

## Acceptance Criteria

- negotiation responses expose richer offer history
- offer submission updates the negotiation row and appends history atomically
- accept and reject are explicit API actions
- the reservation lease remains the finalization gate
- HTTP, MCP, and mobile share the same payload shapes and idempotency behavior
- current state names remain unchanged
- no duplicate reservation is created under concurrent finalization attempts

## Non-Goals

- payment settlement
- cancel semantics unless later approved
- state renaming
- replacing the reservation lease model

## Risks

| Option | Pros | Cons |
| --- | --- | --- |
| Negotiation-row-led history | Better auditability and API clarity | More schema and service work |
| Keep only latest offer | Simple and compact | Weak UX and poor traceability |
| Add cancel now | More complete lifecycle | More state rules and more implementation surface |

## Validation Checklist

- [x] `docs/specs/openapi.yaml` still matches the chosen negotiation shape
- [x] `offer_history` is defined before code changes start
- [x] `accept` and `reject` endpoints are specified before implementation
- [x] `cancel` remains out of scope unless explicitly approved
- [x] state names remain unchanged
- [x] reservation remains the final gate for reveal and finalization

## Next Steps

1. Keep integration coverage for submit/accept/reject and history persistence.
2. Decide whether `cancel` is needed before implementation.
3. Split remaining work into schema, service, and endpoint hardening tasks.
4. Keep OpenAPI, runtime routes, and parity reports aligned in the same review cycle.
