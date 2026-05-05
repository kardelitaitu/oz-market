# Audit Events And Outbox

## Goal

Define how the system records and emits important state changes without mixing delivery concerns into core write logic.

This doc connects:

- auditability
- event delivery
- retry safety
- webhook or push delivery later

## Core Rule

Business state, audit records, and event publication should be related, but not identical.

Recommended split:

- core tables hold canonical entity state
- `audit_events` records who did what and why
- outbox records what must be delivered to downstream consumers

## Why This Separation Helps

| Option | Pros | Cons |
| --- | --- | --- |
| one generic event table only | simple first design | mixes audit, delivery, and debugging concerns |
| separate audit and outbox records | clearer responsibility, safer retries, better operations | slightly more schema and worker design |

Recommendation:

- keep `audit_events` and delivery-oriented outbox records separate
- allow them to share a common event identifier when helpful

## Recommended Records

### `audit_events`

Purpose:

- actor traceability
- support and admin review
- abuse investigation
- compliance-oriented history

Suggested fields:

- `audit_event_id`
- `entity_type`
- `entity_id`
- `action_type`
- `actor_type`
- `actor_id`
- `reason`
- `before_version`
- `after_version`
- `created_at`

### `outbox_events`

Purpose:

- reliable deferred delivery
- webhook or push fan-out
- MCP or mobile notification adapters later

Suggested fields:

- `outbox_event_id`
- `event_type`
- `entity_type`
- `entity_id`
- `entity_version`
- `payload_reference`
- `delivery_status`
- `created_at`
- `delivered_at`

## Transaction Rule

For write paths that mutate business state:

- entity write and audit write should commit together
- outbox record should be created from committed state intent
- delivery workers should publish only after commit

This reduces lost-event risk without making delivery part of the request path.

## When To Write Audit Events

Recommended minimum:

- listing created
- listing archived
- negotiation opened
- offer submitted
- negotiation status changed
- reservation held, released, or expired
- contact reveal requested
- contact reveal approved or rejected
- trust-level changed
- quota override applied
- agent credential revoked

## When To Write Outbox Events

Recommended minimum:

- negotiation status changed
- reservation state changed
- contact reveal approved or rejected
- listing status changed if clients care about active/reserved/sold transitions

## Delivery Rule

- outbox processing should be at-least-once
- consumers should deduplicate by event id
- failed delivery should not roll back committed marketplace state
- read APIs remain the canonical reconciliation path

## Sensitive Data Rule

- audit events should avoid storing raw phone numbers in clear text
- outbox payloads should carry references, not raw secret values
- secret resolution should stay inside privileged service paths only

## Recommended V1 Direction

- implement `audit_events` in the core schema
- design `outbox_events` now even if external webhook delivery is deferred
- keep polling as the first client-facing event consumption path

## Best Next Moves

1. reflect `audit_events` and `outbox_events` in the data-model doc later
2. keep webhook delivery dependent on outbox rather than direct inline callbacks
3. add audit and outbox checks to the server test strategy
