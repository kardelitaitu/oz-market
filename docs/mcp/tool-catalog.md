# MCP Tool Catalog

## Goal

Define the first MCP tool set for desktop agents.

The MCP server should stay thin and call the same backend service logic as HTTP.

## V1 Tool Set

| Tool | Purpose | Required Role |
| --- | --- | --- |
| `create_listing` | create seller listing | `seller_listing_writer` |
| `update_listing_status` | update listing state | `seller_listing_writer` |
| `search_listings` | search indexed listings | `buyer_searcher` or seller-side role |
| `get_listing` | fetch one listing | authenticated client |
| `open_negotiation` | open buyer-side negotiation | `buyer_negotiator` |
| `submit_offer` | submit or counter offer | `buyer_negotiator` or `seller_negotiator` |
| `get_negotiation_status` | fetch negotiation state | authorized participant |
| `request_contact_reveal` | request reveal for reserved negotiation | authorized participant |
| `approve_contact_reveal` | seller-side reveal approval | `seller_contact_reveal_approver` |

## Example Inputs

### `create_listing`

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
  "description": "Good battery health, no major scratches"
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

### `submit_offer`

```json
{
  "negotiation_id": "neg_123",
  "offer_currency": "USD",
  "offer_amount": 440,
  "idempotency_key": "offer-001"
}
```

## Example Outputs

### `create_listing`

```json
{
  "listing_id": "lst_123",
  "status": "active",
  "version": 1,
  "created_at": "2026-05-04T00:00:00Z"
}
```

### `submit_offer`

```json
{
  "negotiation_id": "neg_123",
  "status": "countered",
  "offer_currency": "USD",
  "latest_offer_amount": 440
}
```

## Error Shape

```json
{
  "error": {
    "code": "forbidden",
    "message": "credential cannot approve reveal for this seller"
  }
}
```
