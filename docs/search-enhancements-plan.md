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

**IMPORTANT**: `sort_by` field in `SearchRequest` is **NOT optional** (uses `Default` impl):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchRequest {
    // ... existing fields
    #[serde(default)]
    pub sort_by: SearchSort,  // NOT optional! Has Default impl.
}
```

#### 2.2 Implement Rating Sort in Rust (compare_search_items)
**File**: `backend/server/src/services/search.rs`

**IMPORTANT**: Current architecture uses **Rust sorting** via `compare_search_items()`, not SQL ORDER BY!

Update `compare_search_items()` to handle new sort options:
```rust
pub fn compare_search_items(
    a: &ListingSummary,
    b: &ListingSummary,
    query_terms: &[String],
    sort_by: SearchSort,
) -> Ordering {
    match sort_by {
        SearchSort::Relevance => {
            // ... existing relevance logic
        }
        SearchSort::Newest => {
            // ... existing newest logic
        }
        SearchSort::PriceAsc => {
            // ... existing price_asc logic
        }
        SearchSort::PriceDesc => {
            // ... existing price_desc logic
        }
        // NEW: Rating sort
        SearchSort::RatingHighest => {
            // Sort by seller_rating descending (highest first)
            let rating_a = a.seller_rating.unwrap_or(0.0);
            let rating_b = b.seller_rating.unwrap_or(0.0);
            rating_b.partial_cmp(&rating_a)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.listing_id.cmp(&b.listing_id))
        }
        SearchSort::RatingLowest => {
            // Sort by seller_rating ascending (lowest first)
            let rating_a = a.seller_rating.unwrap_or(0.0);
            let rating_b = b.seller_rating.unwrap_or(0.0);
            rating_a.partial_cmp(&rating_b)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.listing_id.cmp(&b.listing_id))
        }
    }
}
```

**NOTE**: `fetch_rows()` in `listings.rs` calls this function AFTER fetching rows:
```rust
items.sort_by(|a, b| {
    crate::services::search::compare_search_items(a, b, &query_terms, request.sort_by)
});
```

No SQL ORDER BY changes needed! ✅

#### 2.3 Update OpenAPI Spec
```yaml
- name: sort_by
  in: query
  schema:
    $ref: '#/components/schemas/SearchSort'
    default: "relevance"
    # Add to SearchSort enum:
    # - rating_highest
    # - rating_lowest
    # NOTE: sort_by is NOT optional in API contract
    # It uses Default impl (returns "relevance")
```

---

## 3. Search by Seller Name/Rating

### Current State
- ✅ Search by listing fields (product name, description, etc.)
- ❌ Cannot search by seller name
- ❌ Cannot find all listings from a specific seller by name

### Implementation

#### 3.3 Add "seller:" Search Prefix (Recommended - Simple!)

Allow queries like `seller:"Shop Name"` to explicitly search seller names:

```rust
// In fetch_rows(), modify query parsing:
if let Some(query) = &request.query {
    // Check for "seller:" prefix (case-insensitive)
    if query.to_lowercase().starts_with("seller:") {
        let seller_query = query.trim_start_matches("seller:").trim();
        builder.push("s.display_name ILIKE ").push_bind(format!("%{}%", seller_query));
    } else {
        builder.push("l.search_text LIKE ").push_bind(format!("%{}%", query.to_ascii_lowercase()));
    }
}
```

**Why this approach?**
- ✅ No need to modify `listing_index_text()` (which doesn't have access to `seller_name`)
- ✅ No migration needed (no `search_text` rebuild)
- ✅ Simple and fast (uses existing `s.display_name` from LEFT JOIN)
- ✅ Works with current architecture
- ✅ Optional: can add "seller:" prefix search later

---

## 4. Geolocation-Based Search ("Near Me") - UPDATED!

### Requirements:
1. ✅ **More accurate** - Use lat/long coordinates (not just city matching)
2. ✅ **Free** - No paid APIs, use Haversine formula
3. ✅ **Optional** - Listings can opt out (no lat/long = excluded from "near me")
4. ✅ **Simple** - No PostGIS required!

### 4.1 Add Location Fields to Listings (Optional)

**File**: `backend/crates/api-contract/src/listing.rs`

```rust
pub struct ListingLocation {
    pub country_code: String,
    pub country_name: String,
    pub city: String,
    // NEW: Optional coordinates (listing can opt out)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geolocation_opt_out: Option<bool>,  // NEW: Explicit opt-out
}
```

**Logic**:
- If `geolocation_opt_out = Some(true)` → Exclude from "near me" searches
- If `latitude` or `longitude` is `None` → Exclude from "near me" (opted out)
- If both present → Include in distance calculations

---

### 4.2 Add to SearchRequest

**File**: `backend/crates/api-contract/src/listing.rs`

```rust
pub struct SearchRequest {
    // ... existing fields
    
    // NEW: "Near me" search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub near_me: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_latitude: Option<f64>,   // From browser Geolocation API
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_longitude: Option<f64>,  // From browser Geolocation API
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius_km: Option<f64>,  // NEW: Search radius (default: 50km)
}
```

---

### 4.3 Haversine Distance Calculation (No PostGIS!)

**File**: `backend/server/src/repositories/listings.rs` (in `fetch_rows()`)

**✅ SIMPLE APPROACH**: Compute distance inline in WHERE and ORDER BY — no need to add computed column to SELECT!

```rust
if let Some(true) = request.near_me {
    if let (Some(user_lat), Some(user_lon)) = (request.user_latitude, request.user_longitude) {
        if where_added {
            builder.push(" AND ");
        } else {
            builder.push(" WHERE ");
            where_added = true;
        }
        
        // Only include listings that opted in (have coordinates and didn't opt out)
        builder.push("l.latitude IS NOT NULL AND l.longitude IS NOT NULL ");
        builder.push("AND (l.geolocation_opt_out IS NULL OR l.geolocation_opt_out = false) ");
        
        // Calculate distance using Haversine formula (inline in WHERE)
        let radius_km = request.radius_km.unwrap_or(50.0);  // Default: 50km
        
        builder.push("AND (");
        builder.push("  6371 * acos(");  // Earth's radius in km
        builder.push("    cos(radians(").push_bind(user_lat).push(")) * ");
        builder.push("    cos(radians(l.latitude)) * ");
        builder.push("    cos(radians(l.longitude) - radians(").push_bind(user_lon).push(")) + ");
        builder.push("    sin(radians(").push_bind(user_lat).push(")) * ");
        builder.push("    sin(radians(l.latitude))");
        builder.push("  ) <= ").push_bind(radius_km);
        builder.push(")");
        
        // Order by distance (nearest first) - compute inline
        builder.push(" ORDER BY ");
        builder.push("  6371 * acos(");
        builder.push("    cos(radians(").push_bind(user_lat).push(")) * ");
        builder.push("    cos(radians(l.latitude)) * ");
        builder.push("    cos(radians(l.longitude) - radians(").push_bind(user_lon).push(")) + ");
        builder.push("    sin(radians(").push_bind(user_lat).push(")) * ");
        builder.push("    sin(radians(l.latitude))");
        builder.push("  ) ASC, l.listing_id");
    } else {
        // User location not provided - fall back to city/country match
        // (already implemented via existing location filter)
    }
}
```

**Why this approach?**
- ✅ No need to refactor SELECT building (less code change)
- ✅ Slightly slower (computes distance 2 times) but negligible for <1000 results
- ✅ Easier to implement
- ✅ Still uses parameterized queries (`push_bind`) for safety

---

### 4.4 Update OpenAPI Spec

**File**: `docs/specs/openapi.yaml`

```yaml
- name: near_me
  in: query
  schema:
    type: boolean
  description: Prioritize results near the user (requires lat/long)

- name: user_latitude
  in: query
  schema:
    type: number
    description: User's latitude (from browser Geolocation API)

- name: user_longitude
  in: query
  schema:
    type: number
    description: User's longitude (from browser Geolocation API)

- name: radius_km
  in: query
  schema:
    type: number
    minimum: 1
    maximum: 500
    default: 50
    description: Search radius in kilometers (default: 50km)
```

---

### 4.5 Client-Side: Browser Geolocation API

**Frontend/JavaScript**:
```javascript
// Get user's location from browser
if (navigator.geolocation) {
    navigator.geolocation.getCurrentPosition(
        (position) => {
            const lat = position.coords.latitude;
            const lon = position.coords.longitude;
            
            // Send to search API
            fetch(`/v1/listings/search?near_me=true&user_latitude=${lat}&user_longitude=${lon}&radius_km=25`)
                .then(response => response.json())
                .then(data => console.log(data));
        },
        (error) => {
            console.log("Geolocation denied or unavailable");
            // Fall back to city/country search
        }
    );
}
```

---

### 4.6 Database Migration

**File**: `backend/server/migrations/0007_add_coordinates.sql` (NEW)

```sql
ALTER TABLE listings 
ADD COLUMN latitude DECIMAL(10,8),
ADD COLUMN longitude DECIMAL(11,8),
ADD COLUMN geolocation_opt_out BOOLEAN DEFAULT FALSE;

-- Optional: Create index for distance queries
CREATE INDEX idx_listings_coordinates 
ON listings (latitude, longitude) 
WHERE latitude IS NOT NULL AND longitude IS NOT NULL;
```

---

### 4.7 How Listings Opt Out

**Option A**: **Explicit opt-out flag** (recommended)
```sql
UPDATE listings SET geolocation_opt_out = true WHERE listing_id = 'lst_123';
```

**Option B**: **No coordinates** (implicit opt-out)
```rust
// When creating listing, seller can:
let listing = ListingPayload {
    // ... other fields
    location: ListingLocation {
        country_code: "US".to_string(),
        country_name: "United States".to_string(),
        city: "New York".to_string(),
        latitude: None,   // Opt out by not providing coordinates
        longitude: None,
        geolocation_opt_out: Some(true),  // Or explicit opt-out
    },
}
```

---

## 📊 Summary of Changes

| Feature | Implementation | Cost | Accuracy |
|---------|-----------------|------|----------|
| **Distance calc** | Haversine formula in SQL | ✅ Free | ✅ ~99% accurate |
| **User location** | Browser Geolocation API | ✅ Free | ✅ High |
| **Opt out** | `geolocation_opt_out` flag OR no lat/long | ✅ Free | ✅ Flexible |
| **No PostGIS** | Pure SQL calculation | ✅ Free | ✅ Good enough |

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

### Phase C: Seller Name Search (Low Effort, Medium Impact)
1. ✅ Add "seller:" prefix search in `fetch_rows()` (simple approach)
2. ✅ Test searching by seller name: `query="seller:Shop Name"`
3. ✅ No migration needed (uses existing `s.display_name` from LEFT JOIN)

### Phase D: Geolocation Search (Medium Effort, Medium Impact)
1. ✅ Add `near_me`, `user_latitude`, `user_longitude`, `radius_km` to `SearchRequest`
2. ✅ Add `latitude`, `longitude`, `geolocation_opt_out` to `ListingLocation`
3. ✅ Implement Haversine distance inline in WHERE and ORDER BY (no SELECT change!)
4. ✅ No need to refactor SELECT building (simpler approach)
5. ✅ Test with populated database (100k listings)

---

## Files to Modify

| File | Changes |
|------|---------|
| `backend/crates/api-contract/src/listing.rs` | Add fields to `SearchRequest`, add `RatingHighest/Lowest` to `SearchSort`, update `ListingLocation` |
| `backend/server/src/repositories/listings.rs` | Update `fetch_rows()` SQL for new filters, rating sort, geolocation |
| `backend/server/src/services/search.rs` | Update `compare_search_items()` for rating sort (RatingHighest/Lowest) |
| `backend/server/migrations/0007_*.sql` | Migration to add lat/long columns to listings (geolocation) |
| `docs/specs/openapi.yaml` | Add new query parameters to `/listings/search` |
| `backend/server/src/http/actix_handlers.rs` | No changes needed (auto-deserialization) |

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

## 🔍 Pre-Flight Checklist (Before Implementation)

### 1. API Contract (`backend/crates/api-contract/src/listing.rs`)
- [ ] Add `RatingHighest` and `RatingLowest` to `SearchSort` enum
- [ ] Add `min_seller_rating: Option<f64>` to `SearchRequest`
- [ ] Add `verified_sellers_only: Option<bool>` to `SearchRequest`
- [ ] Add `near_me: Option<bool>` to `SearchRequest`
- [ ] Add `user_latitude: Option<f64>` to `SearchRequest`
- [ ] Add `user_longitude: Option<f64>` to `SearchRequest`
- [ ] Add `radius_km: Option<f64>` to `SearchRequest`
- [ ] Add `latitude: Option<f64>` to `ListingLocation`
- [ ] Add `longitude: Option<f64>` to `ListingLocation`
- [ ] Add `geolocation_opt_out: Option<bool>` to `ListingLocation`
- [ ] **NOTE**: `sort_by` is `SearchSort` (NOT `Option<SearchSort>`) - has `Default` impl!

### 2. Repository (`backend/server/src/repositories/listings.rs`)
- [ ] Update `fetch_rows()` to handle new `SearchRequest` fields
- [ ] Add `WHERE s.seller_rating >= $1` for `min_seller_rating`
- [ ] Add `WHERE s.verified_at IS NOT NULL` for `verified_sellers_only`
- [ ] Add Haversine formula **inline** in WHERE and ORDER BY (no SELECT change!)
- [ ] Ensure `where_added` logic handles all new fields
- [ ] Test SQL generation (unit tests)

### 3. Search Service (`backend/server/src/services/search.rs`)
- [ ] Update `compare_search_items()` to handle `RatingHighest/RatingLowest`
- [ ] Add "seller:" prefix check in `fetch_rows()` (no `listing_index_text` change needed!)

### 4. Migration (`backend/server/migrations/0007_*.sql`)
- [ ] `ALTER TABLE listings ADD COLUMN latitude DECIMAL(10,8)`
- [ ] `ALTER TABLE listings ADD COLUMN longitude DECIMAL(11,8)`
- [ ] `ALTER TABLE listings ADD COLUMN geolocation_opt_out BOOLEAN DEFAULT FALSE`
- [ ] Create index: `CREATE INDEX idx_listings_coordinates ON listings (latitude, longitude) WHERE latitude IS NOT NULL`
- [ ] **Test on copy** of production database first!
- [ ] **No need to rebuild `search_text`** (seller search uses "seller:" prefix instead)

### 5. OpenAPI Spec (`docs/specs/openapi.yaml`)
- [ ] Add new parameters to `/listings/search`
- [ ] Add `min_seller_rating`, `verified_sellers_only`
- [ ] Add `near_me`, `user_latitude`, `user_longitude`, `radius_km`
- [ ] Update `SearchSort` enum to include `rating_highest`, `rating_lowest`
- [ ] Add examples for new parameters

### 6. Handlers (`backend/server/src/http/actix_handlers.rs`)
- [ ] Extract `near_me` from `SearchRequest` (optional)
- [ ] No changes needed for other fields (automatically deserialized)

### 7. Testing
- [ ] **Unit tests**: Test `fetch_rows()` SQL generation with new fields
- [ ] **Integration tests**: Test search with 100k listings (maintain ~42k ops/s)
- [ ] **Manual tests**:
     - Search with `min_seller_rating=4.0`
     - Search with `verified_sellers_only=true`
     - Sort by `rating_highest`
     - Search by seller name: `query="seller:Shop Name"`
     - Test `near_me=true` with lat/long
- [ ] **Benchmark**: Ensure performance doesn't degrade!

### 8. Rollback Plan (Just in Case)
- [ ] **Keep migration reversible**: `ALTER TABLE listings DROP COLUMN IF EXISTS latitude`
- [ ] **Feature flag**: Add `enable_geolocation_search` config (optional)
- [ ] **Monitor performance**: Watch ops/s after deployment

---

**✅ Ready to start Phase A (Facetted Search) - Highest ROI!**

