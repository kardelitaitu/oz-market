# Android First User Flow

## Goal

Describe the first useful Android flow without locking into UI-heavy design.

## Flow

1. user signs in
2. app gets backend token
3. user enables AI agent using `openrouter/free`
4. user creates listing or searches listings
5. app negotiates through backend

## Seller Flow

- open `create listing`
- collect canonical listing payload
- wrap it in the `idempotency_key` + `listing` request envelope
- send the envelope to backend
- backend validates auth, quota, and duplicate rules
- app shows `listing_id`, `status`, and `version`

## Buyer Flow

- open `search`
- build canonical search object
- fetch deterministic results
- open listing detail
- start negotiation

## Retry Rule

- preserve the same `idempotency_key` when the app safely retries `create listing` or `open negotiation`
- do not generate a new key for the same logical write attempt

## App Rules

- use the same payload shape as HTTP and MCP
- treat backend as source of truth
- do not let app agent bypass authz or reservation rules
