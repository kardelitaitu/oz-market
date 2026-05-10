# MCP Server

## Why MCP

The marketplace should provide its own `MCP server` so buyer and seller agents can integrate through a constrained tool interface instead of guessing raw HTTP behavior.

This is especially useful for:

- lower-quality agents
- tool-using assistants with weak planning
- integrations that need predictable action names and schemas

## Recommendation

Keep `HTTP JSON API` as the core contract.

Add `MCP` as a thin agent-facing adapter over the same business logic.

This MCP layer is for `desktop agents`.

This gives two integration modes:

| Mode | Pros | Cons |
| --- | --- | --- |
| HTTP JSON API | Fastest, simplest backend contract, best for strong agents and direct integrations | Lower-quality agents may misuse raw endpoints |
| MCP server | Easier for weaker agents, clearer actions, safer workflows | One more surface area to maintain |

## Design Rule

Do not implement business logic twice.

Use this structure:

- `core service layer`
- `HTTP handlers`
- `MCP tools`

Both HTTP and MCP should call the same service functions.

## Recommended V1 MCP Tools

- `create_listing`
- `update_listing_status`
- `search_listings`
- `get_listing`
- `open_negotiation`
- `submit_offer`
- `get_negotiation_status`
- `request_contact_reveal`

## Tool Design Principles

- use simple verbs
- keep inputs small
- return plain structured JSON
- prefer explicit statuses over natural-language ambiguity
- include examples for weak agents
- make invalid transitions fail clearly

## Example MCP Tool Shapes

### `create_listing`

Input:

```json
{
  "idempotency_key": "create-listing-001",
  "listing": {
    "schema_version": "1.0",
    "owner_id": "seller-123",
    "category": "laptop",
    "product_name": "Lenovo ThinkPad T480",
    "condition": "used",
    "price": {
      "currency": "USD",
      "amount": 450
    },
    "location": {
      "country_code": "JP",
      "country_name": "Japan",
      "city": "Osaka"
    },
    "description": "Good battery health",
    "picture_urls": [
      "https://example.com/item.jpg",
      "https://example.com/item2.jpg"
    ],
    "attributes": {
      "brand": "Lenovo",
      "model": "ThinkPad T480"
    }
  }
}
```

Output:

```json
{
  "listing_id": "lst_123",
  "status": "active"
}
```

### `open_negotiation`

Input:

```json
{
  "listing_id": "lst_123",
  "buyer_agent_id": "buyer-agent-1",
  "offer_currency": "USD",
  "offer_amount": 430,
  "idempotency_key": "open-negotiation-001"
}
```

Output:

```json
{
  "negotiation_id": "neg_123",
  "status": "open",
  "offer_currency": "USD",
  "latest_offer_amount": 430
}
```

### `submit_offer`

Input:

```json
{
  "negotiation_id": "neg_123",
  "offer_currency": "USD",
  "offer_amount": 430,
  "idempotency_key": "offer-001"
}
```

Output:

```json
{
  "negotiation_id": "neg_123",
  "status": "countered",
  "offer_currency": "USD",
  "latest_offer_amount": 430
}
```

### `request_contact_reveal`

Input:

```json
{
  "negotiation_id": "neg_123",
  "idempotency_key": "reveal-001"
}
```

Output:

```json
{
  "reveal_id": "rev_123",
  "negotiation_id": "neg_123",
  "reveal_status": "pending"
}
```

### `approve_contact_reveal`

Input:

```json
{
  "reveal_id": "rev_123"
}
```

Output:

```json
{
  "reveal_id": "rev_123",
  "negotiation_id": "neg_123",
  "reveal_status": "approved",
  "revealed_phone_reference": "phone_ref_stub"
}
```

## Event Consumption Model

MCP clients should consume state changes by polling the shared read tools instead of subscribing directly to events in V1.

Recommended polling tools:

- `get_listing`
- `get_negotiation_status`
- `get_contact_reveal`

Model:

- request a write tool
- wait briefly
- poll the matching read tool until the new state is visible
- treat `409` and idempotency replay responses as retry signals, not separate business states
- do not add a separate MCP event stream until the shared delivery model is ready

## Conflict And Retry Examples

- if `create_listing` is replayed with the same `idempotency_key` and same fingerprint, return the original result
- if `create_listing` is replayed with the same `idempotency_key` but a different fingerprint, return a machine-readable conflict
- if `open_negotiation` is retried after a reservation already exists, return the existing reserved response or the conflict reason, not a duplicate reservation
- keep these examples aligned with the shared backend service behavior, not MCP-only logic

## Reliability Rules For MCP

- require schema validation on every tool input
- return machine-readable errors
- expose only allowed transitions
- never reveal phone numbers through generic listing tools
- require explicit approval flow for contact reveal
- require explicit `idempotency_key` for create and replay-sensitive write tools

## Contract Rule

MCP `create_listing` should use the same frozen AI-facing payload as the HTTP API, wrapped in the same create envelope when idempotency is required.

Required MCP listing fields:

- `schema_version`
- `owner_id`
- `category`
- `product_name`
- `condition`
- `price`
- `location`
- `description`

Optional MCP listing fields:

- `picture_urls`
- `attributes`

Required MCP create-envelope field:

- `idempotency_key`

## Performance Rule

MCP should not become a second heavy backend.

| Approach | Pros | Cons |
| --- | --- | --- |
| Thin MCP adapter over core service | Compact codebase, shared logic, lower maintenance risk | Slight adapter work needed |
| Separate MCP business implementation | Full flexibility per integration surface | Duplicated logic, divergence risk, higher maintenance cost |

## Recommended Rollout

1. Define the HTTP service contract first.
2. Extract the shared business logic into a core service module.
3. Map the first 6-8 MCP tools to those service functions.
4. Add examples and strict schemas for weaker agents.
5. Load test HTTP first, then measure MCP overhead separately.

## Best Practice Notes

- keep tool names stable
- version MCP and HTTP together
- publish examples for buyer and seller agents
- keep the MCP surface smaller than the HTTP surface
- prefer deterministic outputs over conversational responses
- do not let MCP behavior diverge from mobile app or server business rules
- keep MCP write-tool envelopes aligned with HTTP when idempotency is required
