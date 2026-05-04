# Internal API Outline

## Goal

Define the first internal operational surface for:

- `admin`
- `support_reviewer`

This is not a frozen public contract. It is a planning outline for `/internal/v1`.

## Boundary Rule

- internal routes should stay out of the public client contract
- internal routes should reuse the same core business logic
- internal routes should require separate privileged scopes
- every internal write should require explicit audit metadata

## Namespace

- base path: `/internal/v1`
- content type: `application/json`
- auth: bearer token with privileged internal scopes

## Candidate Endpoints

### Read-first operational endpoints

- `GET /internal/v1/listings/{listing_id}`
- `GET /internal/v1/negotiations/{negotiation_id}`
- `GET /internal/v1/contact-reveals/{reveal_id}`
- `GET /internal/v1/sellers/{seller_account_id}`
- `GET /internal/v1/audit-events/{entity_type}/{entity_id}`

### Controlled override endpoints

- `POST /internal/v1/listings/{listing_id}/archive`
- `POST /internal/v1/listings/{listing_id}/release-reservation`
- `POST /internal/v1/sellers/{seller_account_id}/trust-level`
- `POST /internal/v1/sellers/{seller_account_id}/quota-override`
- `POST /internal/v1/agent-credentials/{agent_credential_id}/revoke`

## Required Internal Scopes

| Intent | Recommended Scope |
| --- | --- |
| operational read | `support:read` |
| trust or quota override | `admin:moderate` |
| state override | `admin:override` |

## Guardrails

- support reviewers should default to read-only
- override writes should require a `reason`
- override writes should capture before and after state
- internal reads should avoid exposing raw secret material
- internal writes should still respect state-machine safety unless explicit override semantics are documented

## Suggested Override Request Shape

```json
{
  "reason": "manual fraud review outcome",
  "requested_by": "admin_user_123"
}
```

## Suggested Next Move

When internal routes become implementation work:

1. decide whether they live in the public OpenAPI file or a separate internal spec
2. freeze privileged scope names
3. attach audit requirements to every internal write operation
