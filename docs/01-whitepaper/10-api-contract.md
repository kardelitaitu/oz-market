# API Contract

## Goal

Define one canonical contract for:

- `HTTP JSON API`
- `MCP tools`
- `Android` and `iOS` app clients

The same business payload should work across all three surfaces.

## Contract Rules

- the `AI-facing listing JSON` is the source of truth
- `MCP` and mobile clients must not invent alternate field shapes
- meaning changes require a new `schema_version`
- optional fields may be added without breaking old agents
- all responses should be deterministic and machine-readable

## Canonical Listing Payload

```json
{
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
  "picture_urls": [
    "https://example.com/item.jpg",
    "https://example.com/item2.jpg"
  ],
  "description": "Good battery health, no major scratches",
  "attributes": {
    "brand": "Lenovo",
    "model": "ThinkPad T480"
  }
}
```

## Required And Optional Fields

### Required

- `schema_version`
- `owner_id`
- `category`
- `product_name`
- `condition`
- `price.currency`
- `price.amount`
- `location.country_code`
- `location.country_name`
- `location.city`
- `description`

### Optional

- `picture_urls`
- `attributes`

## Enums

### `category`

- `laptop`
- `phone`
- `tablet`
- `desktop`
- `monitor`
- `accessory`
- `camera`
- `audio`
- `gaming`
- `appliance`
- `furniture`
- `vehicle_part`
- `other`

### `condition`

- `new`
- `used`
- `refurbished`

## Transport Mapping

| Surface | Rule |
| --- | --- |
| HTTP | Uses JSON request/response bodies and query parameters where appropriate |
| MCP | Uses the same payload fields inside tool input/output objects |
| Mobile apps | Uses the same payload fields over the HTTP API |

## HTTP Endpoints

### `POST /v1/listings`

Create a new listing.

Use an explicit `idempotency_key` in V1 so agent or mobile retries do not create duplicate listings.

Request body:

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
    "picture_urls": [
      "https://example.com/item.jpg",
      "https://example.com/item2.jpg"
    ],
    "description": "Good battery health, no major scratches",
    "attributes": {
      "brand": "Lenovo",
      "model": "ThinkPad T480"
    }
  }
}
```

Success response:

```json
{
  "listing_id": "lst_123",
  "status": "active",
  "version": 1,
  "created_at": "2026-05-04T00:00:00Z"
}
```

### `GET /v1/listings/{id}`

Fetch one listing.

Success response:

```json
{
  "listing_id": "lst_123",
  "status": "active",
  "version": 1,
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
    "picture_urls": [
      "https://example.com/item.jpg",
      "https://example.com/item2.jpg"
    ],
    "description": "Good battery health, no major scratches",
    "attributes": {
      "brand": "Lenovo",
      "model": "ThinkPad T480"
    }
  }
}
```

### `GET /v1/listings/search`

The canonical search contract is an object. `HTTP GET` maps the same fields into query parameters.

Canonical search object:

```json
{
  "query": "thinkpad",
  "category": "laptop",
  "condition": "used",
  "price": {
    "currency": "USD",
    "min_amount": 300,
    "max_amount": 500
  },
  "location": {
    "country_code": "JP",
    "city": "Osaka"
  },
  "status": "active",
  "sort_by": "relevance",
  "limit": 20,
  "cursor": null
}
```

HTTP query mapping example:

```text
GET /v1/listings/search?query=thinkpad&category=laptop&condition=used&currency=USD&min_amount=300&max_amount=500&country_code=JP&city=Osaka&status=active&sort_by=relevance&limit=20
```

Success response:

```json
{
  "items": [
    {
      "listing_id": "lst_123",
      "status": "active",
      "version": 1,
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
        "picture_urls": [
          "https://example.com/item.jpg"
        ],
        "description": "Good battery health, no major scratches",
        "attributes": {
          "brand": "Lenovo",
          "model": "ThinkPad T480"
        }
      }
    }
  ],
  "next_cursor": "cur_456"
}
```

## Search Rules

- only indexed filter fields are allowed in V1
- `limit` should stay capped, recommended `1..50`
- `cursor` should be used for pagination
- `sort_by` should be deterministic

### `sort_by`

Recommended V1 values:

- `relevance`
- `newest`
- `price_asc`
- `price_desc`

Default behavior:

- if `query` exists, default to `relevance`
- otherwise, default to `newest`
- break ties with `listing_id`

## MCP Tool Mapping

| MCP tool | HTTP equivalent |
| --- | --- |
| `create_listing` | `POST /v1/listings` |
| `search_listings` | `GET /v1/listings/search` |
| `get_listing` | `GET /v1/listings/{listing_id}` |
| `archive_listing` | `POST /internal/v1/listings/{listing_id}/archive` |
| `open_negotiation` | `POST /v1/negotiations` |
| `get_negotiation_status` | `GET /v1/negotiations/{negotiation_id}` |
| `submit_offer` | `POST /v1/negotiations/{negotiation_id}/offers` |
| `request_contact_reveal` | `POST /v1/negotiations/{negotiation_id}/request-contact-reveal` |
| `approve_contact_reveal` | `POST /v1/contact-reveals/{reveal_id}/approve` |
| `get_contact_reveal` | `GET /internal/v1/contact-reveals/{reveal_id}` |

The public desktop-agent MCP catalog should stay smaller than the full HTTP surface.
Internal helpers such as `archive_listing` and `get_contact_reveal` remain server-side unless they are explicitly promoted later.

## MCP Input Examples

### `create_listing`

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
    "picture_urls": [
      "https://example.com/item.jpg",
      "https://example.com/item2.jpg"
    ],
    "description": "Good battery health, no major scratches",
    "attributes": {
      "brand": "Lenovo",
      "model": "ThinkPad T480"
    }
  }
}
```

### `open_negotiation`

```json
{
  "listing_id": "lst_123",
  "buyer_agent_id": "buyer-agent-1",
  "offer_currency": "USD",
  "offer_amount": 430,
  "idempotency_key": "open-negotiation-001"
}
```

### `search_listings`

```json
{
  "query": "thinkpad",
  "category": "laptop",
  "condition": "used",
  "price": {
    "currency": "USD",
    "min_amount": 300,
    "max_amount": 500
  },
  "location": {
    "country_code": "JP",
    "city": "Osaka"
  },
  "status": "active",
  "sort_by": "relevance",
  "limit": 20
}
```

## Error Shape

Use one machine-readable error contract:

```json
{
  "error": {
    "code": "invalid_field",
    "message": "condition must be one of: new, used, refurbished",
    "field": "condition"
  }
}
```

Recommended error codes:

- `invalid_field`
- `missing_field`
- `conflict`
- `not_found`
- `rate_limited`
- `unauthorized`
- `forbidden`

## Best Practice Rules

- keep field names identical across HTTP, MCP, and mobile
- do not expose database-only columns directly unless needed
- keep success responses compact
- keep errors machine-readable
- avoid alternate payload shapes for different clients
- use explicit `idempotency_key` on state-creating and replay-sensitive writes
