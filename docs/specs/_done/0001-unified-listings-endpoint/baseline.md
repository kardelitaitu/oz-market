# Baseline: Unified /v1/listings/{listing_id} Endpoint

## What I Find

### Current State

Currently there are three separate type-specific GET endpoints:

```
GET /v1/product/{listing_id}    → product listing
GET /v1/service/{listing_id}   → service listing
GET /v1/property/{listing_id}  → property listing
```

Each type has its own handler path, making the API surface inconsistent with the create endpoint (`POST /v1/listings` accepts `listing_type` in body).

### Evidence

1. Three separate handler functions with nearly identical logic
2. Clients must know listing type before fetching
3. API contract has separate types: `ProductListing`, `ServiceListing`, `PropertyListing`
4. OpenAPI spec defines three separate paths in `docs/specs/openapi.yaml`

## What I Claim

Merging into a single `/v1/listings/{listing_id}` endpoint will:
- Reduce code duplication in handlers
- Simplify API contract (single `Listing` response type)
- Make API consistent with create/list operations
- Improve developer experience (one endpoint to remember)

The type information is already in the database and response body - embedding it in the URL path adds no value.

## What Is the Proof

1. **Create endpoint is already unified:** `POST /v1/listings` accepts `listing_type` in body - inconsistent with retrieval
2. **Handler logic is nearly identical:** All three endpoints fetch from same repository, only differ by path parsing
3. **Type already in response:** Existing responses already include `listing_type` field - URL path is redundant
4. **Better API design:** Single resource endpoint follows REST best practices (resource, not resource+type in path)
