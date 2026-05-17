# Role Permission Matrix

## Goal

Define a strict, implementation-facing permission matrix for:

- HTTP API
- MCP tools
- mobile app actions

The same permission model must apply across all surfaces.

## Roles

### Seller-side

- `seller_listing_writer`
- `seller_negotiator`
- `seller_contact_reveal_approver`

### Buyer-side

- `buyer_searcher`
- `buyer_negotiator`

### System

- `admin`
- `support_reviewer`

## Scope Rule

Permissions alone are not enough. Every action must also pass:

- authentication
- ownership checks
- reservation checks where relevant
- rate-limit and quota checks on write paths

## Endpoint Matrix

| Action / Endpoint | seller_listing_writer | seller_negotiator | seller_contact_reveal_approver | buyer_searcher | buyer_negotiator | admin | support_reviewer |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `POST /v1/listings` | allow | deny | deny | deny | deny | allow | deny |
| `GET /v1/listings/{listing_id}` | allow | allow | allow | allow | allow | allow | allow |
| `GET /v1/listings/search` | allow | allow | allow | allow | allow | allow | allow |
| `POST /v1/negotiations` | deny | deny | deny | deny | allow | allow | deny |
| `POST /v1/negotiations/{id}/offers` | deny | allow | deny | deny | allow | allow | deny |
| `POST /v1/negotiations/{id}/request-contact-reveal` | deny | allow | deny | deny | allow | allow | deny |
| `POST /v1/contact-reveals/{id}/approve` | deny | deny | allow | deny | deny | allow | deny |

## MCP Tool Matrix

| MCP Tool | seller_listing_writer | seller_negotiator | seller_contact_reveal_approver | buyer_searcher | buyer_negotiator | admin | support_reviewer |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `create_listing` | allow | deny | deny | deny | deny | allow | deny |
| `archive_listing` | deny | deny | deny | deny | deny | allow | deny |
| `search_listings` | allow | allow | allow | allow | allow | allow | allow |
| `get_listing` | allow | allow | allow | allow | allow | allow | allow |
| `open_negotiation` | deny | deny | deny | deny | allow | allow | deny |
| `submit_offer` | deny | allow | deny | deny | allow | allow | deny |
| `get_negotiation_status` | deny | allow | allow | deny | allow | allow | allow |
| `request_contact_reveal` | deny | allow | deny | deny | allow | allow | deny |
| `approve_contact_reveal` | deny | deny | allow | deny | deny | allow | deny |

Public MCP V1 stays on the seller/buyer tools above. Internal admin and support helpers such as `archive_listing` and `get_contact_reveal` stay on the server-side surface.

## Ownership Rules

### Seller-owned actions

These require the acting credential to belong to the same seller account as `owner_id`:

- create listing
- archive listing
- seller counter-offer
- seller reveal approval

### Buyer-owned actions

These require the acting credential to belong to the buyer side of the negotiation:

- open negotiation
- buyer offer submission
- buyer reveal request

## Support And Admin Rules

| Role | Allowed Intent | Not Allowed |
| --- | --- | --- |
| `admin` | operational override, incident recovery, moderation | should still be audited on every privileged action |
| `support_reviewer` | read investigation, support diagnostics | should not create listings, negotiate, or approve reveal |

## Mobile Rule

Mobile app user actions should resolve to backend roles.

Recommended mapping:

- seller app listing flow -> `seller_listing_writer`
- seller app negotiation flow -> `seller_negotiator`
- seller app reveal approval flow -> `seller_contact_reveal_approver`
- buyer app search flow -> `buyer_searcher`
- buyer app negotiation flow -> `buyer_negotiator`

## Conflict And Reservation Rule

Permission is necessary but not sufficient.

Even an allowed role must still fail when:

- listing ownership does not match
- reservation lease does not match
- version check fails
- quota or rate limit is exceeded

## Replay And Idempotency Rule

Permission also does not remove replay-safety requirements.

Required V1 behavior:

- `POST /v1/listings` requires an envelope-level `idempotency_key`
- `POST /v1/negotiations` requires an envelope-level `idempotency_key`
- `POST /v1/negotiations/{id}/offers` requires `idempotency_key`
- `POST /v1/negotiations/{id}/request-contact-reveal` requires `idempotency_key`

Even an allowed role must still fail or safely replay when:

- the same create/open request is retried with the same `idempotency_key`
- the same actor replays a previously accepted replay-sensitive write
- the same transition is retried after the state already advanced

## Recommended Error Codes

- `unauthorized`
- `forbidden`
- `owner_mismatch`
- `reservation_conflict`
- `version_conflict`
- `quota_exceeded`
- `rate_limited`

## Best Next Moves

1. map each endpoint handler to one required role set
2. add ownership checks to every write-path service
3. encode this matrix into authz tests before implementation grows
