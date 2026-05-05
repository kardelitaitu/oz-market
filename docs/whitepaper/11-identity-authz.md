# Identity And Authz

## Goal

Define the trust model for:

- seller onboarding
- buyer and seller agents
- HTTP API access
- MCP access
- mobile app access

The system should trust `seller identity` first, then `agent credentials` acting on behalf of that seller.

## Core Rule

Do not trust `hardware_id` as the main identity anchor.

Primary trust should come from:

- verified seller account
- signed agent credential
- permission checks
- rate limits and quotas
- abuse scoring

## Main Entities

### `seller_account`

Represents the real platform principal.

Suggested fields:

- `seller_account_id`
- `status`
- `trust_level`
- `created_at`
- `updated_at`

### `agent_credential`

Represents one agent allowed to act for a seller.

Suggested fields:

- `agent_credential_id`
- `seller_account_id`
- `agent_type`
- `status`
- `public_key_id` or `token_id`
- `created_at`
- `updated_at`

### `agent_session`

Optional runtime session or token layer.

Suggested fields:

- `agent_session_id`
- `agent_credential_id`
- `issued_at`
- `expires_at`
- `revoked_at`

## Trust Model

| Layer | Purpose | Recommendation |
| --- | --- | --- |
| seller account | primary platform identity | required |
| agent credential | proves an agent acts for a seller | required |
| session/token | short-lived runtime auth | required |
| hardware/device signal | optional abuse evidence | optional only |

## Seller Identity Rule

Every listing owner should map to a real `seller_account_id`.

`owner_id` in the AI-facing payload should be treated as the seller identifier exposed to the marketplace logic, and should resolve to a verified internal seller account.

Recommended rule:

- external payload uses `owner_id`
- internal system maps `owner_id` to `seller_account_id`
- create/update actions fail if the caller is not authorized for that owner

## V1 Seller Onboarding Policy

Start with a verified-account model:

- each seller must have a verified `seller_account`
- new sellers start in a low-trust state with tight quotas
- trusted seller actions require a short-lived agent credential
- risky actions can trigger manual review or `trust_review_required`
- onboarding should stay simple enough for small sellers, but never skip ownership checks

## Agent Credential Rule

Every non-human agent should authenticate with its own credential.

Recommended properties:

- credential belongs to exactly one seller account
- credential can be revoked independently
- credential can have a role and scope
- credential use is fully auditable

Scope shape and token-claim details should be defined explicitly in `21-auth-scopes-and-claims.md`.

## Recommended Roles

### Seller-side roles

- `seller_listing_writer`
- `seller_negotiator`
- `seller_contact_reveal_approver`

### Buyer-side roles

- `buyer_searcher`
- `buyer_negotiator`

### System roles

- `admin`
- `support_reviewer`

## Permission Rules

### Listing permissions

- only seller-linked credentials may create listings for that seller
- only seller-linked credentials may update that seller’s listing status
- search access may be broader than listing-write access

### Negotiation permissions

- buyer-linked credentials may open negotiations
- seller-linked credentials may accept, reject, or counter on owned listings
- only the reserved negotiation may continue into reveal/finalization

### Contact reveal permissions

- only authorized seller-side credential may approve reveal
- only authorized buyer-side reserved negotiation may receive reveal result
- generic listing reads must never expose private contact data

## Authentication Options

| Option | Pros | Cons |
| --- | --- | --- |
| signed API tokens | simple, compact, easy first step | token leakage risk if long-lived |
| public/private key signing | stronger non-repudiation and credential separation | more implementation work |
| OAuth-style delegated auth | strong ecosystem pattern | more complexity than needed for V1 |

## Recommendation

Start with:

- short-lived signed tokens for runtime auth
- separately managed agent credentials
- revocation support
- audit logging for every privileged action

## Mobile App Rule

Mobile apps should authenticate the `user` first, then obtain app-scoped agent access for that user’s agent actions.

Recommended split:

- user logs into mobile app
- app requests short-lived backend token
- app-side AI agent uses backend-approved token path
- app must not bypass seller/account permission checks

## MCP Rule

MCP is not a special-trust channel.

MCP tools must enforce the same:

- authentication
- seller/account ownership checks
- role checks
- rate limits
- abuse controls

## Anti-Abuse Trust Controls

Use these controls together:

- seller onboarding trust level
- agent credential issuance policy
- per-seller quotas
- per-agent quotas
- per-IP and per-token rate limits
- duplicate listing fingerprinting
- anomaly detection
- optional `hardware_id` as abuse signal only

## Trust-Level Progression

Use a simple progression for V1:

1. `new` - verified seller account exists, but quotas are low and abuse checks are strict
2. `verified` - seller has passed the first trust gate and can use normal write access
3. `trusted` - seller has a stable history and can get higher quotas or fewer manual checks
4. `restricted` - seller lost privileges because of abuse, repeated conflicts, or policy violations

## Suggested Failure Codes

- `unauthorized`
- `forbidden`
- `owner_mismatch`
- `credential_revoked`
- `rate_limited`
- `quota_exceeded`
- `trust_review_required`

## Best Next Moves

1. Add `seller_account_id` and `agent_credential_id` to the internal data model.
2. Decide whether V1 auth uses signed tokens only or key-based agent credentials.
3. Define exact role-to-endpoint permissions.
4. Keep trust-level rules aligned with the V1 onboarding policy above.
