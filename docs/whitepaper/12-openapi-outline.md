# OpenAPI Outline

## Goal

Turn the frozen marketplace contract into an implementation-oriented API outline.

This is not a full OpenAPI file yet. It is the schema and endpoint plan to guide the first spec.

## API Version

- base path: `/v1`
- content type: `application/json`
- auth: bearer token

## Core Schemas

### `ListingPayload`

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
    "https://example.com/item.jpg"
  ],
  "description": "Good battery health, no major scratches",
  "attributes": {
    "brand": "Lenovo",
    "model": "ThinkPad T480"
  }
}
```

### `ListingSummary`

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
      "https://example.com/item.jpg"
    ],
    "description": "Good battery health, no major scratches",
    "attributes": {
      "brand": "Lenovo",
      "model": "ThinkPad T480"
    }
  }
}
```

### `SearchRequest`

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

### `ErrorResponse`

```json
{
  "error": {
    "code": "invalid_field",
    "message": "condition must be one of: new, used, refurbished",
    "field": "condition"
  }
}
```

## Endpoints

### `POST /v1/listings`

- purpose: create listing
- auth: seller-linked credential required
- request body: create envelope with `idempotency_key` plus `ListingPayload`
- success: `201 Created`

Response:

```json
{
  "listing_id": "lst_123",
  "status": "active",
  "version": 1,
  "created_at": "2026-05-04T00:00:00Z"
}
```

### `GET /v1/listings/{listing_id}`

- purpose: fetch one listing
- auth: authenticated client
- success: `200 OK`
- response body: `ListingSummary`

### `GET /v1/listings/search`

- purpose: indexed listing search
- auth: authenticated client
- input: query params mapped from `SearchRequest`
- success: `200 OK`

Response:

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

## Negotiation Endpoint Outline

### `POST /v1/negotiations`

- purpose: open negotiation
- auth: buyer-linked credential required
- request body:

```json
{
  "listing_id": "lst_123",
  "buyer_agent_id": "buyer-agent-1",
  "offer_currency": "USD",
  "offer_amount": 430,
  "idempotency_key": "open-negotiation-001"
}
```

### `POST /v1/negotiations/{negotiation_id}/offers`

- purpose: submit counter-offer or updated offer
- auth: authorized buyer or seller credential
- request body:

```json
{
  "offer_currency": "USD",
  "offer_amount": 440,
  "idempotency_key": "offer-001"
}
```

### `POST /v1/negotiations/{negotiation_id}/request-contact-reveal`

- purpose: request reveal for reserved negotiation
- auth: authorized negotiation participant
- request body:

```json
{
  "idempotency_key": "reveal-001"
}
```

### `POST /v1/contact-reveals/{reveal_id}/approve`

- purpose: seller-side reveal approval
- auth: seller-side authorized credential

## Response And Error Rules

- use machine-readable errors only
- keep success bodies compact
- return `409 Conflict` for stale version or reservation conflicts
- return `429 Too Many Requests` for rate-limit and quota control
- require explicit `idempotency_key` for create and replay-sensitive write paths in V1

## Suggested Status Codes

| Status | Usage |
| --- | --- |
| `200` | successful read |
| `201` | successful create |
| `400` | invalid input |
| `401` | unauthenticated |
| `403` | authenticated but not allowed |
| `404` | missing resource |
| `409` | version or reservation conflict |
| `429` | rate limited or quota controlled |

## Best Next Moves

1. Convert this outline into a real OpenAPI YAML file.
2. Define shared reusable schema components for `Price`, `Location`, and `ErrorResponse`.
3. Add auth headers and permission notes per endpoint.
4. Add negotiation and reservation response schemas.
