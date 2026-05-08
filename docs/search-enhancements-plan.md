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

Add SQL with **Haversine formula** (free, accurate distance calculation):

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
        
        // Calculate distance using Haversine formula
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
        
        // Order by distance (nearest first)
        builder.push(" ORDER BY distance_km ASC, l.listing_id");
    } else {
        // User location not provided - fall back to city/country match
        // (already implemented via existing location filter)
    }
}
```

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
