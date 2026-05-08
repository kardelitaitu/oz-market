# Search Enhancements Plan

## Overview

Enhance the current search functionality with:
1. **Faceted search** (filter by seller rating, condition, price range)
2. **Advanced sorting** (relevance, price, date, seller rating)
3. **Seller-based search** (search by seller name/rating)
4. **Geolocation-based search** ("near me" with radius)
5. **Search suggestions/autocomplete** (optional future)

---

## 1. Faceted Search

### Current State
- ✅ Full-text search via `search_text` (trigram index)
- ✅ Filter by: category, condition, status, price range, location
- ❌ Missing: **seller rating filter**, **seller verification filter**

### Implementation

#### 1.1 Add Seller Rating Filter
**File**: `backend/server/src/repositories/listings.rs` (in `fetch_rows()`)

```rust
// Add to SearchRequest (api-contract):
pub struct SearchRequest {
    // ... existing fields
    pub min_seller_rating: Option<f64>,  // NEW: Filter by minimum seller rating (1.0-5.0)
    pub verified_sellers_only: Option<bool>,  // NEW: Only show verified sellers
}
```

**SQL modification** in `fetch_rows()`:
```rust
if let Some(min_rating) = request.min_seller_rating {
    if where_added {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        where_added = true;
    }
    builder.push("s.seller_rating >= ").push_bind(min_rating);
}

if let Some(true) = request.verified_sellers_only {
    if where_added {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        where_added = true;
    }
    builder.push("s.verified_at IS NOT NULL");
}
```

#### 1.2 Update API Contract
**File**: `backend/crates/api-contract/src/listing.rs`

Add to `SearchRequest`:
```rust
pub struct SearchRequest {
    // ... existing fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_seller_rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_sellers_only: Option<bool>,
}
```

#### 1.3 Update OpenAPI Spec
**File**: `docs/specs/openapi.yaml`

Add to `/listings/search` parameters:
```yaml
- name: min_seller_rating
  in: query
  schema:
    type: number
    minimum: 1.0
    maximum: 5.0
    description: Filter by minimum seller rating (1.0-5.0)
- name: verified_sellers_only
  in: query
  schema:
    type: boolean
    description: Only show verified sellers
```

---

## 2. Advanced Sorting Options

### Current State
- ✅ Default sort: relevance (if query) or newest (if no query)
- ✅ Supported: `relevance`, `newest`, `price_asc`, `price_desc`

### Implementation

#### 2.1 Add Rating Sort
**File**: `backend/crates/api-contract/src/listing.rs`

Add to `SearchSort` enum:
```rust
pub enum SearchSort {
    Relevance,
    Newest,
    PriceAsc,
    PriceDesc,
    RatingHighest,  // NEW
    RatingLowest,   // NEW
}
```

#### 2.2 Implement Rating Sort in SQL
**File**: `backend/server/src/repositories/listings.rs`

Modify `fetch_rows()` to handle new sort options:
```rust
match request.sort_by {
    Some(SearchSort::RatingHighest) => {
        builder.push(" ORDER BY s.seller_rating DESC NULLS LAST, l.listing_id");
    }
    Some(SearchSort::RatingLowest) => {
        builder.push(" ORDER BY s.seller_rating ASC NULLS FIRST, l.listing_id");
    }
    // ... existing sort options
}
```

#### 2.3 Update OpenAPI Spec
```yaml
- name: sort_by
  in: query
  schema:
    $ref: '#/components/schemas/SearchSort'
    # Add to SearchSort enum:
    # - rating_highest
    # - rating_lowest
```

---

## 3. Search by Seller Name/Rating

### Current State
- ✅ Search by listing fields (product name, description, etc.)
- ❌ Cannot search by seller name
- ❌ Cannot find all listings from a specific seller by name

### Implementation

#### 3.1 Add Seller Name to Search Index
**File**: `backend/server/src/services/search.rs`

Update `listing_index_text()` to include seller name:
```rust
pub fn listing_index_text(listing: &ListingPayload) -> String {
    let mut parts = vec![
        listing.product_name.clone(),
        listing.description.clone(),
        listing.location.city.clone(),
        listing.location.country_name.clone(),
    ];
    
    // Add seller name if available
    if let Some(seller_name) = &listing.seller_name {
        parts.push(seller_name.clone());
    }
    
    parts.join(" ")
}
```

#### 3.2 Update Trigram Index
**File**: `backend/server/migrations/0007_add_seller_name_to_search.sql` (NEW)

```sql
-- Update search_text to include seller name
UPDATE listings l
SET search_text = (
    SELECT CONCAT_WS(' ', 
        l2.product_name, 
        l2.description, 
        l2.city, 
        l2.country_name,
        s.display_name
    )
    FROM listings l2
    LEFT JOIN seller_accounts s ON l2.owner_id = s.owner_id
    WHERE l2.listing_id = l.listing_id
);

-- Rebuild trigram index (optional, for performance)
DROP INDEX IF EXISTS idx_listings_search_text;
CREATE INDEX idx_listings_search_text ON listings USING GIN (search_text gin_trgm_ops);
```

#### 3.3 Add "seller:" Search Prefix (Optional)
Allow queries like `seller:"Shop Name"` to explicitly search seller names:

```rust
// In fetch_rows(), modify query parsing:
if let Some(query) = &request.query {
    // Check for "seller:" prefix
    if query.starts_with("seller:") {
        let seller_query = query.trim_start_matches("seller:");
        builder.push("s.display_name ILIKE ").push_bind(format!("%{}%", seller_query));
    } else {
        builder.push("l.search_text LIKE ").push_bind(format!("%{}%", query.to_ascii_lowercase()));
    }
}
```

---

## 4. Geolocation-Based Search ("Near Me")

### Current State
- ✅ Search by country_code and city
- ❌ Cannot search by "near me" with radius
- ❌ No distance calculation

### Implementation

#### 4.1 Add PostGIS Extension (Optional - Heavy)
For true geolocation, need PostGIS. For simplicity, use **city + country** matching:

#### 4.2 Simple "Near Me" Implementation
**File**: `backend/crates/api-contract/src/listing.rs`

Add to `SearchRequest`:
```rust
pub struct SearchRequest {
    // ... existing fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub near_me: Option<bool>,  // NEW: Use requestor's location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,   // NEW: For "near me" (future)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,  // NEW: For "near me" (future)
}
```

#### 4.3 Get User's Location from Claims
**File**: `backend/server/src/http/actix_handlers.rs`

Extract location from claims or request:
```rust
// In search_listings handler, check for "near_me" flag
if request.near_me.unwrap_or(false) {
    // Get user's location from claims (if stored) or use default
    // For now, just prioritize results from same city/country
}
```

**Simpler approach**: Add `location_preference` to claims or use cookie.

#### 4.4 Update OpenAPI Spec
```yaml
- name: near_me
  in: query
  schema:
    type: boolean
    description: Prioritize results near the user (uses requestor's location)
- name: latitude
  in: query
  schema:
    type: number
    description: Latitude for distance calculation (future)
- name: longitude
  in: query
  schema:
    type: number
    description: Longitude for distance calculation (future)
```

---

## 5. Search Suggestions/Autocomplete (Optional Future)

### Concept
- Provide real-time search suggestions as user types
- Based on: popular queries, product names, categories

### Implementation (Future)
1. Create `search_suggestions` table
2. Track popular search queries
3. Endpoint: `GET /v1/search/suggestions?q=lap` → returns `["laptop", "latitude 5410", ...]`

---

## Implementation Order

### Phase A: Faceted Search (High Impact, Medium Effort)
1. ✅ Add `min_seller_rating` and `verified_sellers_only` to `SearchRequest`
2. ✅ Update `fetch_rows()` SQL in `listings.rs`
3. ✅ Update OpenAPI spec
4. ✅ Test with populated database (100k listings)

### Phase B: Advanced Sorting (Low Effort, High Impact)
1. ✅ Add `RatingHighest` and `RatingLowest` to `SearchSort`
2. ✅ Implement rating sort in SQL
3. ✅ Update OpenAPI spec

### Phase C: Seller Name Search (Medium Effort, Medium Impact)
1. ✅ Update `listing_index_text()` to include seller name
2. ✅ Create migration to rebuild `search_text` with seller names
3. ✅ Test searching by seller name

### Phase D: Geolocation Search (High Effort, Medium Impact)
1. ✅ Add `near_me` flag to `SearchRequest`
2. ✅ Implement simple city/country prioritization
3. ❌ (Future) Add PostGIS for true distance calculation

---

## Files to Modify

| File | Changes |
|------|---------|
| `backend/crates/api-contract/src/listing.rs` | Add fields to `SearchRequest`, add `RatingHighest/Lowest` to `SearchSort` |
| `backend/server/src/repositories/listings.rs` | Update `fetch_rows()` SQL for new filters and sorting |
| `backend/server/src/services/search.rs` | Update `listing_index_text()` to include seller name |
| `backend/server/migrations/0007_*.sql` | Migration to update `search_text` and rebuild index |
| `docs/specs/openapi.yaml` | Add new query parameters to `/listings/search` |
| `backend/server/src/http/actix_handlers.rs` | Extract `near_me` from request (optional) |

---

## Testing Plan

1. **Unit Tests**: Test new SQL generation in `fetch_rows()`
2. **Integration Tests**: Test search with 100k listings
3. **Benchmark**: Ensure new filters don't degrade performance (maintain ~42k ops/s)
4. **Manual Testing**:
   - Search with `min_seller_rating=4.0`
   - Search with `verified_sellers_only=true`
   - Sort by `rating_highest`
   - Search by seller name: `query="seller:Shop Name"`
   - Test `near_me=true`

---

## Expected Performance Impact

- **Faceted search**: Minimal impact (indexed columns, simple WHERE clauses)
- **Rating sort**: Small impact (sorting by numeric column with index)
- **Seller name search**: Medium impact (trigram index on `search_text` still fast)
- **Geolocation**: Depends on implementation (city match = fast, PostGIS = medium)

**Goal**: Maintain **~40,000 ops/s** with new search features!

---

## Next Steps

1. **Start with Phase A** (Faceted Search) - highest ROI
2. **Implement Phase B** (Advanced Sorting) - quick win
3. **Consider Phase C** (Seller Name) - if needed
4. **Defer Phase D** (Geolocation) - unless specifically requested

---

**Document Status**: Draft  
**Created**: 2026-05-08  
**Author**: pi  
**Priority**: High (improves search experience significantly)
