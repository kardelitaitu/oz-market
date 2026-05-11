# Plan: Unified /v1/listings/{listing_id} Endpoint

## What Is the Solution

### Step 1: Backend Handler Merge

1. Create new unified `get_listing` handler in `backend/server/src/handlers/listings.rs`
2. Handler accepts `listing_id` from path, fetches listing from repository
3. Response includes `listing_type` field extracted from entity
4. Old handlers remain but route to new logic (feature flag: `UNIFIED_LISTINGS_ENDPOINT`)
5. HTTP and MCP surfaces call the same listing service logic (no duplicated business rules)

**New handler signature:**
```rust
async fn get_listing(
    State(state): State<AppState>,
    Path(listing_id): Path<String>,
) -> Result<Json<Listing>, StatusCode>
```

### Step 2: API Contract Update

1. Keep internal type-specific types (ProductListing, ServiceListing, PropertyListing) for type-specific fields
2. Create unified `Listing` response type that wraps internal types
3. Add `listing_type` field to `Listing` as discriminator
4. Update route registration to OpenAPI path `/listings/{listing_id}` (external path `/v1/listings/{listing_id}`)

### Step 3: OpenAPI Spec Update

Update `docs/specs/openapi.yaml`:
- Keep `/listings/{listing_id}` path with GET operation
- Keep old type-specific paths out of the frozen OpenAPI contract
- Mark as breaking change

### Step 4: Old Endpoint Deprecation

Old endpoints (`/v1/product/{listing_id}`, `/v1/service/{listing_id}`, `/v1/property/{listing_id}`) return:
```http
HTTP/1.1 301 Moved Permanently
Location: /v1/listings/{listing_id}
Deprecation: true
Sunset: Sat, 01 Jun 2026 00:00:00 GMT
```

**Timeline:**
- Week 1: New endpoint live, old return Deprecation header
- Week 4: Old endpoints return 301 redirect
- Month 2: Remove old endpoint handlers

### Step 5: Mobile Client Updates

1. Replace 3 endpoint methods with single `getListing(id:)`
2. Parse `listing_type` from response for local routing
3. Update cached URL references

### Step 6: Documentation

- Update `docs/DOCS-README.md`
- Update `docs/01-whitepaper/10-api-contract.md`
- Add breaking change notice
