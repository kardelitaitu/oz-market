# Data Model

## Principle

Keep the `product listing` minimal, but do not force the entire system into one JSON object. Reliability needs a small amount of structured operational data around the listing, even in a compact codebase.

## Public Listing Payload

This remains the core listing content:

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

`picture_urls` are external-only. The service never stores image files.

## Frozen V1 AI Contract

The `01-overview.md` payload is the canonical V1 AI-facing contract.

Database design should support that contract, but should not force every query to parse nested JSON.

## Recommended Minimal Tables

### `listings`

- `id`
- `owner_id`
- `status`
- `schema_version`
- `category`
- `condition`
- `country_code`
- `country_name`
- `city`
- `product_name`
- `currency`
- `price_amount`
- `search_text`
- `version`
- `product_payload_json`
- `created_at`
- `updated_at`

### `negotiations`

- `id`
- `listing_id`
- `buyer_agent_id`
- `seller_agent_id`
- `status`
- `offer_currency`
- `latest_offer_amount`
- `final_offer_amount`
- `version`
- `created_at`
- `updated_at`

### `negotiation_messages`

- `id`
- `negotiation_id`
- `sender_agent_id`
- `message_type`
- `message_payload_json`
- `created_at`

### `contact_reveals`

- `id`
- `negotiation_id`
- `reveal_status`
- `revealed_phone_reference`
- `requested_at`
- `approved_at`

### `audit_events`

- `id`
- `entity_type`
- `entity_id`
- `actor_id`
- `event_type`
- `event_payload_json`
- `created_at`

### `outbox_events`

- `id`
- `event_type`
- `entity_type`
- `entity_id`
- `entity_version`
- `payload_reference`
- `delivery_status`
- `created_at`
- `delivered_at`

## Why Not Store Everything In One Collection

| Approach | Pros | Cons |
| --- | --- | --- |
| Single listing JSON only | Very simple at first | Weak negotiation tracking, weak auditability, hard state control |
| Minimal structured tables + JSON payload | Still simple, but safer and easier to scale | Slightly more schema design |
| Minimal structured tables + audit and outbox separation | Better reliability, delivery safety, and operations | More schema and worker planning |

## Data Separation Rules

- `Public data`: product fields needed for search and negotiation
- `Private data`: phone number and any direct contact data
- `System data`: statuses, audit events, outbox events, timestamps, actor references

## Performance-Oriented Modeling Notes

- Promote high-filter fields such as `category`, `condition`, `country_code`, `city`, `product_name`, and `status` into indexed columns
- Keep `description` and optional fields inside the JSON payload
- Use `JSONB` for flexible metadata, not for the entire hot-path record
- Avoid joins on the read-heavy listing path when possible
- Use append-only message rows for negotiation history instead of rewriting large blobs
- Keep negotiation money fields explicit so offer logic does not parse nested blobs
- Keep audit and outbox rows append-only so retries and investigations stay predictable

## Recommended PostgreSQL Shape

Use `PostgreSQL` with a hybrid model:

- typed columns for hot reads and indexed filters
- `JSONB` for flexible payload fields
- small row size on the main listing path

### Example Direction

`listings` should keep these as typed columns:

- `id`
- `owner_id`
- `status`
- `schema_version`
- `category`
- `condition`
- `country_code`
- `country_name`
- `city`
- `product_name`
- `currency`
- `price_amount`
- `search_text`
- `version`
- `created_at`
- `updated_at`

`listings.product_payload_json` should only keep flexible fields such as:

- `description`
- `picture_urls`
- `attributes`
- future optional metadata

This keeps the API payload flexible without making database filtering slow or unpredictable.

## Audit And Outbox Direction

`audit_events` and `outbox_events` should stay separate.

Recommended rule:

- `audit_events` tracks who did what and why
- `outbox_events` tracks what must be delivered asynchronously
- business-state writes should commit with audit writes
- outbox delivery should happen after committed state exists

This keeps operations, retries, and event delivery easier to reason about.

## Search-Oriented Indexing Direction

Use `multi-dimensional indexing` on the main listing path:

- `status`
- `category`
- `condition`
- `country_code`
- `country_name`
- `city`
- `currency`
- `price_amount`
- `product_name`
- `search_text`

Recommended first indexes:

- composite index on `(status, category, condition, country_code, city)`
- composite index on `(status, currency, price_amount)`
- full-text or trigram index on `search_text`
- partial indexes limited to `active` listings when possible

This gives fast search on the common agent filters without needing a second search service immediately.

## Required And Optional Fields

### Required In AI Contract

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

### Optional In AI Contract

- `picture_urls`
- `attributes`

## Suggested Enums

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

### Listing status

- `draft`
- `active`
- `reserved`
- `sold`
- `archived`

### Negotiation status

- `open`
- `countered`
- `near_close`
- `contact_requested`
- `contact_revealed`
- `closed`
- `cancelled`

### Contact reveal status

- `pending`
- `approved`
- `rejected`
- `expired`
