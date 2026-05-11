# Decisions

## Decision Log

| Decision | Value | Reason |
| --- | --- | --- |
| Offer history | API exposes richer append-only history | Clients need visible negotiation history |
| Offer write model | Negotiation-row-led | Keeps state and history together |
| Finalization endpoints | `accept` and `reject` | Clear user action mapping |
| Cancel | deferred | Avoid extra surface until needed |
| State names | unchanged | Prevent churn before approval |
| Reservation | final gate | Preserves safety and double-sell protection |
| Single-negotiation enforcement | DB-level uniqueness via deterministic `neg_{listing_id}` + primary-key conflict | Deterministic race resolution without adding status-index complexity yet |

## Notes

- Keep HTTP, MCP, and mobile aligned.
- Keep idempotency on replay-sensitive writes.
- Keep reservation checks on the finalization path.

## DB Enforcement Decision (2026-05-11)

| Option | Pros | Cons |
| --- | --- | --- |
| App-level only (checks in service/repo code) | Flexible for future lifecycle variants | Weaker against race bugs and future call-site drift |
| DB partial unique index on `(listing_id)` for active statuses | Strong invariant at storage layer | Adds migration complexity and status-coupled index semantics |
| Deterministic `neg_{listing_id}` ID + PK conflict (chosen) | DB-enforced uniqueness now, minimal schema churn | One negotiation lifecycle per listing unless ID strategy changes later |

Chosen rule:

- enforce single negotiation per listing through deterministic ID generation (`neg_{listing_id}`) and database primary-key conflict handling.
- keep reservation checks as finalization safety guard.
- defer partial-unique-by-status index until reopen semantics are introduced.

## OpenAPI Parity Check (2026-05-11)

- `docs/specs/openapi.yaml` contains `/negotiations`, `/negotiations/{negotiation_id}`, `/negotiations/{negotiation_id}/offers`, `/negotiations/{negotiation_id}/accept`, and `/negotiations/{negotiation_id}/reject`.
- `docs/specs/openapi.yaml` includes `AcceptNegotiationRequest`, `RejectNegotiationRequest`, and `offer_history` in `NegotiationResponse`.
- Runtime routes in HTTP/TCP and shared app methods are aligned to the same payload shapes.
