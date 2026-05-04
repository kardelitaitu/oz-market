# Admin And Support Surfaces

## Goal

Define how internal operational actions should exist without polluting the public marketplace contract.

The system already has `admin` and `support_reviewer` roles. Their surfaces should be explicit before ad hoc internal endpoints appear.

## Core Rule

Public marketplace actions and internal operational actions should share core business logic, but not necessarily the same route surface.

## Surface Options

| Option | Pros | Cons |
| --- | --- | --- |
| mix admin endpoints into public `/v1` | one spec, one transport, simple first implementation | public spec gets noisy and privilege boundaries blur |
| separate internal namespace | clearer trust boundary, easier audit, safer long-term | slightly more documentation and routing work |
| separate internal service later | strongest isolation | more operational complexity too early |

Recommendation:

- keep one server process for now
- use a separate internal route namespace, such as `/internal/v1`
- keep public and internal operations in separate spec sections or separate specs later

## Public Vs Internal Boundary

### Public surface

- listing create, read, search
- negotiation open, read, offer submit
- contact reveal request and approval

### Internal surface

- trust review actions
- abuse review actions
- quota override actions
- reservation-release override actions
- reveal investigation reads
- support diagnostics

## Recommended Internal Endpoints

These are planning placeholders, not frozen public API:

- `GET /internal/v1/listings/{listing_id}`
- `GET /internal/v1/negotiations/{negotiation_id}`
- `GET /internal/v1/contact-reveals/{reveal_id}`
- `POST /internal/v1/listings/{listing_id}/archive`
- `POST /internal/v1/listings/{listing_id}/release-reservation`
- `POST /internal/v1/sellers/{seller_account_id}/trust-level`
- `POST /internal/v1/sellers/{seller_account_id}/quota-override`

## Operational Rules

- every internal write action must be audited
- support roles should default to read-only
- admin override actions should require explicit reason metadata
- internal actions should still obey state-machine safety unless an override path is explicitly documented

## Override Rule

An override should not mean arbitrary mutation.

Recommended override guardrails:

- privileged scope required
- reason field required
- actor identity logged
- before/after state logged
- optional second-review policy for sensitive actions later

## Spec Strategy

| Option | Pros | Cons |
| --- | --- | --- |
| public and internal in one OpenAPI file | easy single-source maintenance early | bigger spec, easier accidental client confusion |
| separate public and internal specs | cleaner client contracts | more upkeep and sync discipline |

Recommendation:

- document internal endpoints in whitepaper now
- keep public spec clean first
- add internal spec only when internal endpoints actually start implementation

## Support Reviewer Rule

`support_reviewer` should be able to:

- inspect listing state
- inspect negotiation state
- inspect reveal state
- inspect audit and rate-limit outcomes

`support_reviewer` should not be able to:

- create listings
- submit offers
- approve reveals
- silently alter quota or trust state

## Best Next Moves

1. decide whether `/internal/v1` stays in the same binary or behind a separate access gateway
2. document required internal scopes alongside these routes
3. keep internal endpoints out of the public client contract until implementation begins
