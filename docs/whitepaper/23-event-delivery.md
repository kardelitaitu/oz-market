# Event Delivery

## Goal

Define how clients learn about state changes for:

- negotiation status changes
- reservation changes
- contact reveal approvals or rejections
- abuse or quota outcomes where needed

The system should support reliable state awareness without forcing every client to guess when to poll.

## Core Rule

The canonical source of truth is still the read API.

Event delivery should help clients react faster, but it should not replace:

- `GET /v1/listings/{id}`
- `GET /v1/negotiations/{id}`
- future reveal-status reads when needed

## Delivery Options

| Option | Pros | Cons |
| --- | --- | --- |
| polling only | simplest, most reliable, easiest to reason about | slower updates, more read load |
| webhooks | efficient for server-to-server agents | delivery retries and signature verification add complexity |
| SSE or WebSocket | good for live UI updates | more connection management cost |
| event log plus optional delivery adapters | flexible, auditable, supports multiple consumers | more design work upfront |

Recommendation:

- keep an internal append-only event log
- support polling first for all clients
- add signed webhooks for server-to-server consumers later
- keep mobile push and live-stream adapters as later layers over the same event source

## Recommended V1 Strategy

### Required in V1

- canonical polling reads
- short polling guidance for negotiation status
- append-only internal event log for important state transitions

### Deferred but designed for

- signed webhook subscriptions
- mobile push-notification adapters
- MCP subscription or long-poll helper flow

## Event Types

Suggested internal event types:

- `listing.created`
- `listing.status_changed`
- `negotiation.opened`
- `negotiation.offer_submitted`
- `negotiation.status_changed`
- `reservation.held`
- `reservation.released`
- `reservation.expired`
- `contact_reveal.requested`
- `contact_reveal.approved`
- `contact_reveal.rejected`

## Delivery Rules

- events should be append-only
- events should carry entity id, event type, version, and timestamp
- delivery retries should never mutate business state
- consumers should treat delivery as at-least-once
- state reads should resolve ambiguity after receiving an event

## Client Strategy

### MCP and desktop agents

- start with polling for negotiation status and listing state
- later support webhook-style callbacks only if agent hosts can receive them reliably

### Mobile apps

- use polling first for active negotiation screens
- later map internal events to mobile push notifications through app infrastructure

### Server-to-server integrations

- best candidate for signed webhooks
- should validate webhook signatures and re-fetch canonical state when needed

## Reliability Rules

- event publication failure must not corrupt core write transactions
- if transactional outbox is used later, it should publish from committed state only
- event payloads should stay compact and reference entity ids instead of duplicating large objects

## Suggested Event Envelope

```json
{
  "event_id": "evt_123",
  "event_type": "negotiation.status_changed",
  "entity_type": "negotiation",
  "entity_id": "neg_123",
  "entity_version": 4,
  "occurred_at": "2026-05-04T00:00:00Z"
}
```

## Best Next Moves

1. keep polling as the first guaranteed integration path
2. define a future webhook signature model before adding delivery endpoints
3. tie event publication to audit or outbox design in the server docs later
