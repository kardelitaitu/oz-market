# Concurrency And Reservations

## Goal

Prevent:

- double-sell
- conflicting accept states
- duplicate reveal approvals
- fake listing spam
- abusive agent retries

The system must stay deterministic even when many buyer and seller agents act at the same time.

## Core Rule

A listing can have many `open negotiations`, but only one active `reservation lease` at a time.

That means:

- many buyers may negotiate in parallel
- only one buyer may hold the short finalization window
- contact reveal and final acceptance must check the active reservation first

## Recommended State Model

### Listing state

- `active`
- `reserved`
- `sold`
- `archived`

### Negotiation state

- `open`
- `countered`
- `near_close`
- `reserved`
- `contact_requested`
- `contact_revealed`
- `closed`
- `cancelled`

### Reservation state

- `held`
- `expired`
- `released`
- `converted`

## Recommended Minimal Table

### `reservation_leases`

- `id`
- `listing_id`
- `negotiation_id`
- `buyer_agent_id`
- `lease_status`
- `lease_version`
- `held_until`
- `created_at`
- `updated_at`

## Reservation Lease Rule

Use a short-lived lease instead of an indefinite reservation.

Recommended behavior:

- seller accepts a near-close buyer
- marketplace creates a reservation lease
- listing moves from `active` to `reserved`
- only that negotiation may request final contact reveal or purchase completion
- lease expires automatically if not completed in time

Recommended first lease window:

- `5 to 15 minutes` for manual seller/buyer handoff

## Versioning Rule

Every state-changing write should use optimistic version checks.

Recommended versioned entities:

- `listings`
- `negotiations`
- `reservation_leases`

Recommended write pattern:

- client sends current entity version
- update succeeds only if stored version matches
- server increments version on success
- stale writes fail with a clear conflict error

This prevents late agent retries from overwriting newer decisions.

## Acceptance Rule

The system should never allow a direct jump from `open` to `sold`.

Recommended acceptance flow:

1. buyer submits or confirms final offer
2. seller accepts
3. system creates reservation lease
4. system blocks other negotiations from entering reserved/final state
5. contact reveal happens only for the reserved negotiation
6. listing becomes `sold` only after explicit completion event

## Double-Sell Prevention

Use all of these controls together:

- unique active reservation per listing
- optimistic versioning on listing and negotiation writes
- transactional state transition on accept/reserve
- lease expiration job
- idempotency keys for accept and reveal actions

### Database Direction

Recommended relational constraints:

- at most one `held` reservation lease per `listing_id`
- negotiation cannot enter `reserved` unless listing is `reserved`
- contact reveal cannot be approved without active matching lease

## Conflict Handling

When two buyers race for the same listing:

- first successful reservation transaction wins
- later attempts receive a conflict response
- losing negotiations stay `open` or become `cancelled_by_conflict`

Agents must see explicit machine-readable conflict reasons.

## Retry Safety

Common unsafe behavior:

- same offer sent many times
- same reveal request sent many times
- same acceptance callback repeated after timeout

Required controls:

- idempotency keys on all finalization actions
- conflict-safe updates
- bounded retry windows
- audit event for duplicate/replayed finalization attempts

## Fake Listing And Spam Strategy

Do not rely on `hardware_id` alone.

`hardware_id` is weak because:

- many agents run in cloud or containers
- it is easy to rotate or spoof in some environments
- privacy and portability problems appear quickly
- good agents may share infrastructure

## Recommended Anti-Spam Stack

Use layered controls:

- verified seller account
- signed agent credential tied to seller account
- rate limits per seller, agent, IP, and API key
- listing creation quota for new sellers
- stricter limits until seller trust increases
- duplicate listing detection
- challenge or manual review on suspicious creation bursts

## Device Identity Comparison

| Option | Pros | Cons |
| --- | --- | --- |
| `hardware_id` required | Harder for very low-effort abuse | Weak in cloud environments, spoofable risk, hurts legitimate portability |
| `hardware_id` as optional signal | Extra abuse signal for scoring | Not strong enough as primary defense |
| seller identity + agent credential + quotas | Reliable, auditable, works across cloud and local agents | Requires stronger auth/onboarding design |

## Recommendation

Use `hardware_id` only as an optional abuse signal, not as the main trust anchor.

Primary trust should come from:

- seller account identity
- agent credential issuance
- listing quotas
- rate limits
- behavioral abuse scoring

## Listing Abuse Checks

Before allowing `create_listing`, check:

- seller account status
- seller current listing quota
- recent create-listing burst rate
- duplicate product fingerprint
- repeated identical `picture_urls` or description patterns
- location and price anomaly rules

## Duplicate Listing Fingerprint

Create a normalized fingerprint from fields such as:

- `owner_id`
- normalized `product_name`
- normalized `city`
- normalized `category`
- normalized `condition`
- normalized `currency`
- normalized `price.amount`

Use it to:

- block exact duplicates
- flag near-duplicates for review
- reduce marketplace pollution

## MCP And HTTP Rule

Both `HTTP API` and `MCP tools` must pass through the same:

- reservation logic
- version checks
- quota checks
- abuse detection

MCP must not become a bypass around listing safety.

## Best Next Moves

1. Add `version` fields to `listings` and `negotiations` in the schema plan.
2. Add `reservation_leases` to the data model.
3. Define the exact `accept -> reserve -> reveal -> complete` state machine.
4. Define seller onboarding and initial listing quota policy.
5. Decide whether `hardware_id` is collected at all, and if yes, keep it optional.
