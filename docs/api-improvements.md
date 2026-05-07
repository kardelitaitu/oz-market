# API Improvements Plan

> **Purpose**: Enhance the marketplace API with richer listing data and better seller information.
> **Scope**: V1 bridge service — no images, no full e-commerce, just metadata + negotiation bridge.
> **Goal**: More useful listings for AI agents and mobile clients.

---

## Table of Contents

1. [Current State](#current-state)
2. [Phase A: Enhance Listing Data](#phase-a-enhance-listing-data)
3. [Phase B: Expose Seller Summary](#phase-b-expose-seller-summary)
4. [Phase C: API Improvements](#phase-c-api-improvements)
5. [Implementation Order](#implementation-order)
6. [Backward Compatibility](#backward-compatibility)

---

## Current State

### What We Have ✅

**ListingPayload** (create/update):
```rust
pub struct ListingPayload {
    pub schema_version: String,
    pub owner_id: String,
    pub category: Category,
    pub product_name: String,
    pub condition: Condition,
    pub price: Price,
    pub location: ListingLocation,
    pub picture_urls: Vec<String>,  // Note: whitepaper says NO image uploads!
    pub description: String,
    pub attributes: Option<Value>,  // Free-form JSON
}
```

**ListingSummary** (read):
```rust
pub struct ListingSummary {
    pub listing_id: String,
    pub status: ListingStatus,
    pub version: u64,
    pub listing: ListingPayload,
}
```

**SearchRequest**:
- `query`, `category`, `condition`, `price`, `location`, `status`, `sort_by`, `limit`, `cursor`

### What's Missing ⚠️

1. **No quantity field** — sellers can't say "I have 3 of these"
2. **No SKU** — sellers can't track inventory
3. **No shipping info** — local pickup? shipping available?
4. **No condition details** — "like new", "minor scratches"
5. **No seller info in summary** — buyers can't see seller name/rating
6. **OpenAPI spec lacks examples** — hard for clients to understand
7. **No caching headers** — HTTP caching alongside Moka

---

## Phase A: Enhance Listing Data

### A.1: Add Fields to `ListingPayload`

**New fields** (all optional for backward compatibility):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListingPayload {
    // ... existing fields ...
    
    // NEW: Marketplace fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,  // Seller's inventory SKU
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u32>,  // Number available (default: 1)
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_info: Option<ShippingInfo>,  // Shipping details
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_details: Option<String>,  // "like new", "minor scratches"
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_notes: Option<String>,  // Additional info for buyers
}
```

### A.2: New `ShippingInfo` Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShippingInfo {
    pub local_pickup: bool,  // Can pick up locally
    pub shipping_available: bool,  // Can ship
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_cost: Option<Price>,  // Shipping cost if applicable
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_regions: Option<Vec<String>>,  // ["JP-OSAKA", "JP-TOKYO"]
}
```

### A.3: Database Migration

**PostgreSQL changes** (`listings` table):

```sql
ALTER TABLE listings ADD COLUMN IF NOT EXISTS sku VARCHAR(100);
ALTER TABLE listings ADD COLUMN IF NOT EXISTS quantity INTEGER DEFAULT 1;
ALTER TABLE listings ADD COLUMN IF NOT EXISTS shipping_info JSONB;
ALTER TABLE listings ADD COLUMN IF NOT EXISTS condition_details TEXT;
ALTER TABLE listings ADD COLUMN IF NOT EXISTS seller_notes TEXT;
```

**Update `ListingRow`** in `models/db.rs`:

```rust
pub struct ListingRow {
    // ... existing fields ...
    pub sku: Option<String>,
    pub quantity: i32,
    pub shipping_info: Option<Value>,
    pub condition_details: Option<String>,
    pub seller_notes: Option<String>,
}
```

### A.4: Update Repository

**In `repositories/listings.rs`**:

1. Update `insert_listing` to handle new fields
2. Update `row_to_summary` to map new fields
3. Update `summary_to_row` (if exists)

---

## Phase B: Expose Seller Summary

### B.1: Add Seller Info to `ListingSummary`

**New fields** (read-only, fetched from seller account):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListingSummary {
    pub listing_id: String,
    pub status: ListingStatus,
    pub version: u64,
    pub listing: ListingPayload,
    
    // NEW: Seller summary (read-only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_name: Option<String>,  // From seller_accounts table
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_rating: Option<f32>,  // 0.0-5.0, from reviews (future)
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_verified: Option<bool>,  // From seller_accounts.trust_level
}
```

### B.2: Update `get_listing` and `search_listings`

**In `repositories/listings.rs`**:

```rust
async fn get_listing(
    &self,
    listing_id: &str,
) -> Result<Option<ListingSummary>, RepositoryError> {
    // 1. Fetch listing from listings table
    let listing = /* existing query */;
    
    // 2. Fetch seller info from seller_accounts table
    if let Some(ref listing) = listing {
        let seller_info = sqlx::query(
            "SELECT display_name, trust_level FROM seller_accounts WHERE seller_id = $1"
        )
        .bind(&listing.listing.owner_id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = seller_info {
            // Add seller_name, seller_verified to ListingSummary
        }
    }
    
    Ok(listing)
}
```

**Note**: For search, this would be N+1 queries. Consider:
- Option B.1: Batch fetch seller info for search results
- Option B.2: Add seller info via JOIN in search query
- Option B.3: Keep as-is for search, only add for single `get_listing`

**Recommendation**: Option B.3 (simplest, search performance matters more)

---

## Phase C: API Improvements

### C.1: Add Examples to OpenAPI Spec

**In `docs/specs/openapi.yaml`**:

```yaml
components:
  schemas:
    ListingPayload:
      type: object
      properties:
        sku:
          type: string
          description: Seller's inventory SKU
          example: "LAPTOP-001"
        quantity:
          type: integer
          description: Number of identical items available
          default: 1
          example: 3
        shipping_info:
          $ref: '#/components/schemas/ShippingInfo'
        condition_details:
          type: string
          description: Granular condition description
          example: "Like new, only used for 2 months"
        seller_notes:
          type: string
          description: Additional notes for buyers
          example: "Includes original box and charger"
    
    ShippingInfo:
      type: object
      properties:
        local_pickup:
          type: boolean
          example: true
        shipping_available:
          type: boolean
          example: false
        shipping_cost:
          $ref: '#/components/schemas/Price'
        shipping_regions:
          type: array
          items:
            type: string
          example: ["JP-OSAKA", "JP-KYOTO"]
```

### C.2: Add HTTP Caching Headers

**In `actix_handlers.rs`**:

```rust
use actix_web::http::header::{LastModified, ETag};
use chrono::Utc;

pub async fn get_listing(/* ... */) -> impl Responder {
    // ... existing logic ...
    
    if let Ok(Some(listing)) = result {
        let etag = format!("\"{}\"", md5::compute(listing.listing_id.as_bytes()));
        return HttpResponse::Ok()
            .insert_header(LastModified(/* from updated_at */))
            .insert_header(ETag(etag))
            .json(listing);
    }
    // ...
}
```

### C.3: Improve Error Responses

**Ensure all error responses have consistent format**:

```yaml
components:
  responses:
    BadRequest:
      description: Bad request
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
    Unauthorized:
      description: Unauthorized
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
    NotFound:
      description: Resource not found
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

  schemas:
    ErrorResponse:
      type: object
      required: [error_code, message]
      properties:
        error_code:
          type: string
          example: "NOT_FOUND"
        message:
          type: string
          example: "Listing not found"
        request_id:
          type: string
          description: For tracing (from tracing span)
          example: "abc123"
        timestamp:
          type: string
          format: date-time
          example: "2026-05-07T20:00:00Z"
```

---

## Implementation Order

### Step 1: Update API Contract (Today)

1. ✅ Add new fields to `ListingPayload` in `backend/crates/api-contract/src/listing.rs`
2. ✅ Add `ShippingInfo` struct
3. ✅ Add seller fields to `ListingSummary`
4. ✅ Update `Default` implementation for new fields

### Step 2: Update OpenAPI Spec (Today)

1. ✅ Add new schemas (`ShippingInfo`, updated `ListingPayload`)
2. ✅ Add examples to all schemas
3. ✅ Improve error response schemas

### Step 3: Database Migration (Today)

1. ✅ Create migration SQL file in `backend/server/migrations/`
2. ✅ Update `ListingRow` in `backend/server/src/models/db.rs`
3. ✅ Update `row_to_summary` in `repositories/listings.rs`

### Step 4: Update Server (Today)

1. ✅ Update `insert_listing` to handle new fields
2. ✅ Update `get_listing` to return seller info (optional)
3. ✅ Update `search_listings` (maybe skip seller info for performance)
4. ✅ Test with `http_bench.rs` to ensure no performance regression

### Step 5: Add HTTP Caching (Optional, Later)

1. ⚫ Add `Last-Modified` / `ETag` headers
2. ⚫ Update Moka cache to respect these headers

---

## Backward Compatibility

### ✅ Fully Backward Compatible

- **All new fields are `Option<T>`** — existing clients won't break
- **Default values** — `quantity` defaults to 1 if not specified
- **Database migration** — `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` (safe to re-run)
- **API contract** — no breaking changes, only additions

### Migration Path

**Old client** (without new fields):
```json
{
  "listing": {
    "product_name": "ThinkPad T480",
    "price": {"currency": "USD", "amount": 450.0}
    // No sku, quantity, etc. — still works!
  }
}
```

**New client** (with new fields):
```json
{
  "listing": {
    "product_name": "ThinkPad T480",
    "price": {"currency": "USD", "amount": 450.0},
    "sku": "LAPTOP-001",
    "quantity": 3,
    "shipping_info": {"local_pickup": true, "shipping_available": false}
  }
}
```

---

## Expected Impact

### Performance

| Change | Impact | Notes |
|--------|--------|-------|
| Add new fields to `ListingPayload` | **None** | Optional fields, serialized conditionally |
| Add seller info to `get_listing` | **Minimal** | 1 extra query to `seller_accounts` |
| Add seller info to `search_listings` | **⚠️ Avoid** | N+1 query problem, use batch or skip |
| HTTP caching headers | **Positive** | Reduces repeated requests |

### Business Value

| Feature | Value |
|---------|-------|
| `sku` | Sellers can track inventory |
| `quantity` | Sellers can sell multiple identical items |
| `shipping_info` | Buyers know shipping options |
| `condition_details` | Better search/discovery |
| `seller_name` | Trust-building for buyers |
| `seller_verified` | Quality signal for buyers |

---

## Next Steps After Implementation

1. **Test** — Ensure all 37 tests still pass
2. **Benchmark** — Verify no performance regression (maintain 5,000+ ops/s)
3. **Document** — Update `docs/whitepaper/` if needed
4. **Mobile client** — Use new fields in Android/iOS apps
5. **MCP server** — Expose new fields to AI agents

---

**Document Status**: Ready for implementation  
**Last Updated**: 2026-05-07  
**Author**: pi  
**Depends On**: Phase 1 complete ✅, Production hardening complete ✅  
**Next Action**: Implement Step 1 (Update API contract)
