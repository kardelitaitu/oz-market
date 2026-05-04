# State Machines

## Goal

Define strict transition rules for:

- listings
- negotiations
- reservation leases
- contact reveals

The system should prefer explicit transitions over inferred behavior.

## Listing State Machine

### States

- `draft`
- `active`
- `reserved`
- `sold`
- `archived`

### Allowed Transitions

| From | To | Rule |
| --- | --- | --- |
| `draft` | `active` | listing passes validation and publish action |
| `active` | `reserved` | matching reservation lease is created |
| `reserved` | `active` | lease expires or is released |
| `reserved` | `sold` | explicit completion event after reserved flow |
| `active` | `archived` | seller or system archive action |
| `reserved` | `archived` | only after release or administrative override |
| `sold` | `archived` | optional lifecycle cleanup |

### Invalid Transitions

- `draft -> sold`
- `active -> sold`
- `sold -> active`

## Negotiation State Machine

### States

- `open`
- `countered`
- `near_close`
- `reserved`
- `contact_requested`
- `contact_revealed`
- `closed`
- `cancelled`

### Allowed Transitions

| From | To | Rule |
| --- | --- | --- |
| `open` | `countered` | seller or buyer responds with new offer |
| `open` | `near_close` | policy or seller marks near-close |
| `countered` | `countered` | another counter-offer |
| `countered` | `near_close` | price gap or seller signal qualifies |
| `near_close` | `reserved` | reservation lease is created |
| `reserved` | `contact_requested` | authorized participant requests reveal |
| `contact_requested` | `contact_revealed` | seller-side approval succeeds |
| `contact_revealed` | `closed` | explicit completion event |
| `open` | `cancelled` | participant or system cancellation |
| `countered` | `cancelled` | participant or system cancellation |
| `near_close` | `cancelled` | participant or system cancellation |

### Invalid Transitions

- `open -> contact_revealed`
- `countered -> closed`
- `near_close -> sold`
- `cancelled -> open`

## Reservation Lease State Machine

### States

- `held`
- `expired`
- `released`
- `converted`

### Allowed Transitions

| From | To | Rule |
| --- | --- | --- |
| `held` | `expired` | lease timeout job fires |
| `held` | `released` | seller or system releases reservation |
| `held` | `converted` | deal proceeds to completed close path |

### Rules

- only one `held` lease may exist per listing
- `listing.status = reserved` must imply exactly one matching `held` lease
- `negotiation.status = reserved` must imply matching held lease

## Contact Reveal State Machine

### States

- `pending`
- `approved`
- `rejected`
- `expired`

### Allowed Transitions

| From | To | Rule |
| --- | --- | --- |
| `pending` | `approved` | authorized seller-side approval |
| `pending` | `rejected` | seller-side rejection or policy rejection |
| `pending` | `expired` | reveal request times out |

### Rules

- reveal request requires matching reserved negotiation
- approval requires authorized seller-side credential
- approved reveal should return only controlled phone reference

## Cross-Entity Invariants

These must always stay true:

1. at most one active reservation lease per listing
2. listing cannot be `sold` without prior `reserved`
3. reveal cannot be `approved` without matching held reservation
4. closed negotiation should not return to active states
5. stale version writes must fail instead of forcing transition

## Transition Guard Types

Every transition should validate:

- role permission
- ownership
- current entity version
- reservation lease match
- rate limit or quota if applicable
- idempotency key for replay-sensitive actions

## Recommended Conflict Responses

- `version_conflict`
- `reservation_conflict`
- `invalid_transition`
- `owner_mismatch`

## Best Next Moves

1. encode these state machines into domain-service tests
2. map each API endpoint to exactly one transition intent
3. ensure background jobs only perform allowed timeout transitions
