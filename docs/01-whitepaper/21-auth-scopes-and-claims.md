# Auth Scopes And Claims

## Goal

Define a strict token and credential model for:

- HTTP API
- MCP access
- Android app
- iOS app

Roles describe business permission. Scopes and claims describe what the credential can actually do at runtime.

## Core Rule

Every authenticated request should carry both:

- identity claims
- scope claims

Role checks should not depend on raw string matching alone.

`idempotency_key` is not an identity claim.

It is a transport-level request field used for safe retries on create or replay-sensitive writes.

## Recommended Claim Shape

Suggested token claims:

- `sub`: subject identifier
- `actor_type`: `user`, `agent_credential`, or `system`
- `seller_account_id`: present for seller-linked credentials
- `buyer_account_id`: present for buyer-linked credentials when applicable
- `agent_credential_id`: present for agent flows
- `scopes`: list of granted scopes
- `session_id`: short-lived session identifier
- `exp`: expiry timestamp
- `iat`: issue timestamp
- `jti`: token identifier

## Recommended Scope Model

Use narrow action-oriented scopes.

### Listing scopes

- `listing:create`
- `listing:read`
- `listing:search`
- `listing:status:update`

### Negotiation scopes

- `negotiation:create`
- `negotiation:read`
- `negotiation:offer:submit`
- `negotiation:reveal:request`

### Reveal scopes

- `reveal:approve`
- `reveal:read`

### Internal scopes

- `admin:moderate`
- `admin:override`
- `support:read`

## Role To Scope Mapping

| Role | Recommended Scopes |
| --- | --- |
| `seller_listing_writer` | `listing:create`, `listing:read`, `listing:search`, `listing:status:update` |
| `seller_negotiator` | `listing:read`, `listing:search`, `negotiation:read`, `negotiation:offer:submit`, `negotiation:reveal:request` |
| `seller_contact_reveal_approver` | `listing:read`, `negotiation:read`, `reveal:approve`, `reveal:read` |
| `buyer_searcher` | `listing:read`, `listing:search` |
| `buyer_negotiator` | `listing:read`, `listing:search`, `negotiation:create`, `negotiation:read`, `negotiation:offer:submit`, `negotiation:reveal:request` |
| `admin` | internal administrative scopes only, plus audited override powers |
| `support_reviewer` | `listing:read`, `negotiation:read`, `reveal:read`, `support:read` |

## Scope Evaluation Rule

Permission should require all of:

- valid authentication
- required scope
- matching owner or participant claim where applicable
- valid entity state
- valid reservation lease when applicable

Scope alone should never bypass ownership or reservation rules.

## Surface Rules

### HTTP API

- bearer token should include runtime scopes
- endpoint docs should declare required scopes
- `idempotency_key` should travel in the request body for operations that require replay safety

### MCP

- MCP tool execution should resolve to the same scope checks as HTTP
- MCP must not invent privileged scopes outside the core auth model
- MCP tools should pass `idempotency_key` as input data, not derive it from claims

### Mobile

- mobile user session should mint short-lived app tokens
- app-side AI agent actions should use backend-issued scoped tokens only
- mobile retry logic should preserve the same `idempotency_key` across safe retries

## Identity Claims Vs Transport Controls

| Concern | Example | Where It Belongs |
| --- | --- | --- |
| identity | `seller_account_id`, `agent_credential_id` | token claims |
| permission | `listing:create`, `negotiation:create` | token scopes |
| retry safety | `idempotency_key` | request body or transport envelope |

## Short-Lived Token Rule

| Option | Pros | Cons |
| --- | --- | --- |
| broad long-lived tokens | simpler to issue | weak revocation and larger blast radius |
| short-lived scoped tokens | safer, easier audit, better least privilege | more token refresh work |

Recommendation:

- use short-lived scoped tokens
- keep refresh or reissue server-controlled
- keep privileged scopes rare and separately auditable

## Recommended Failure Codes

- `unauthorized`
- `forbidden`
- `credential_revoked`
- `owner_mismatch`
- `reservation_conflict`
- `trust_review_required`

## Example

Example seller-side scoped token claims:

```json
{
  "sub": "agent_credential:acred_123",
  "actor_type": "agent_credential",
  "seller_account_id": "seller_account_123",
  "agent_credential_id": "acred_123",
  "scopes": [
    "listing:create",
    "listing:read",
    "listing:search"
  ],
  "session_id": "sess_123",
  "iat": 1777852800,
  "exp": 1777853700,
  "jti": "tok_123"
}
```

## Best Next Moves

1. annotate `docs/specs/openapi.yaml` with required scopes per operation
2. define the token issuer and revocation model in server docs later
3. encode role-to-scope mappings into authz tests before transport code expands
