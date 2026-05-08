# Marketplace Expansion Plan: Adding Services & Property Categories

## Current State (as of 2026-05-08)

The marketplace currently supports **only product listings** via the `listings` table/endpoint.

### Existing Listing Schema (Product-Focused):
- `title`, `description`, `price`, `currency`
- `category` (currently used for product categories like "electronics", "furniture")
- `location` (city/region)
- `listing_type` (currently always "product")
- Supports: create, search, get, negotiate, contact reveal

### What Works Today:
✅ Product listings (physical goods)
✅ Search with filters (price, category, location)
✅ Negotiation flow
✅ Review system
✅ AI prompt caching for search
✅ Performance: 42,000+ ops/s

---

## Expansion Goal

Add **two new major categories** to the marketplace:

### 1. **Services** 🛠️
Services are **non-physical offerings** (labor, consulting, digital services).

**Types**:
| Type | Description | Examples |
|------|-------------|----------|
| `local` | In-person, location-based services | Plumbing, house cleaning, electrician |
| `online` | Remote/digital services | Tutoring, freelance programming, graphic design |

**Key Differences from Products**:
- No physical inventory
- Often priced **per hour** or **per project** (not per unit)
- May have **availability schedules** (days/times)
- May require **qualifications/certifications**
- `local` services require `location`; `online` services may be location-independent

### 2. **Property** 🏠
Properties are **real estate** (land, buildings, houses, apartments) for **rent or sale**.

**Sub-Types**:
| Type | Description | Examples |
|------|-------------|----------|
| `building` | Commercial/industrial structures | Office spaces, warehouses, retail shops |
| `house` | Single-family homes, townhouses | Detached houses, duplexes |
| `apartment` | Multi-unit residential | Apartments, condos, lofts |
| `land` | Empty plots/land | Residential lots, agricultural land, vacant plots |

**Transaction Types**:
- **Rent** (short-term/long-term leases)
- **Sale** (permanent ownership transfer)

---

## Proposed Data Model Changes

### New `listing_type` Enum Values
Currently: `"product"` only.

**Proposed**:
```rust
pub enum ListingType {
    Product,  // Existing (physical goods)
    Service,  // NEW: Labor, consulting, digital services
    Property, // NEW: Real estate (rent/sale)
}
```

### Service Listings: Additional Fields
```rust
pub struct ServiceListing {
    // Inherited from base Listing:
    // title, description, price, currency, location, etc.
    
    // Service-specific:
    pub service_type: ServiceType,  // "local" or "online"
    pub hourly_rate: Option<f64>,  // If priced per hour
    pub project_rate: Option<f64>,  // If priced per project
    pub availability: Option<Vec<DaySchedule>>,  // Days/times available (for local)
    pub qualifications: Option<Vec<String>>,  // Certifications, degrees
    pub service_radius_km: Option<i32>,  // For local services (travel radius)
}
```

### Property Listings: Additional Fields
```rust
pub struct PropertyListing {
    // Inherited from base Listing
    
    // Property-specific:
    pub property_transaction_type: PropertyTransactionType, // "rent" or "sale"
    pub property_sub_type: PropertySubType, // "building", "house", "apartment", "land"
    pub area_sqm: Option<f64>,  // Area in square meters
    pub bedrooms: Option<i32>,  // For house/apartment (0 for studio)
    pub bathrooms: Option<i32>,  // For house/apartment
    pub year_built: Option<i32>,  // For building/house/apartment
    pub lot_size_sqm: Option<f64>,  // For land
    pub zoning: Option<String>,  // For land (residential, commercial, agricultural)
}
```

---

## API Changes Required

### 1. Update `CreateListingRequest`
Add conditional fields based on `listing_type`:

```json
// Product (existing)
{
  "listing_type": "product",
  "title": "ThinkPad X1",
  "price": 800.00,
  "category": "electronics"
}

// Service - Online (NEW)
{
  "listing_type": "service",
  "title": "Math Tutoring",
  "service_type": "online",
  "hourly_rate": 50.00,
  "qualifications": ["Teaching License", "Math Degree"],
  "availability": [{"day": "Monday", "slots": ["09:00-12:00", "14:00-17:00"]}]
}

// Service - Local (NEW)
{
  "listing_type": "service",
  "title": "House Cleaning",
  "service_type": "local",
  "location": "New York, NY",
  "hourly_rate": 30.00,
  "service_radius_km": 20
}

// Property - Apartment (NEW)
{
  "listing_type": "property",
  "title": "2BR Apartment for Rent",
  "property_transaction_type": "rent",
  "property_sub_type": "apartment",
  "bedrooms": 2,
  "bathrooms": 1,
  "area_sqm": 85.5,
  "price": 1200.00,
  "currency": "USD"
}

// Property - House (NEW)
{
  "listing_type": "property",
  "title": "Detached House for Sale",
  "property_transaction_type": "sale",
  "property_sub_type": "house",
  "bedrooms": 4,
  "bathrooms": 2,
  "area_sqm": 200.0,
  "lot_size_sqm": 500.0,
  "price": 350000.00
}

// Property - Land (NEW)
{
  "listing_type": "property",
  "title": "Vacant Lot for Development",
  "property_transaction_type": "sale",
  "property_sub_type": "land",
  "lot_size_sqm": 1000.0,
  "zoning": "residential"
}
```

### 2. Update Search Endpoint
Add new filter parameters:

| Parameter | Type | Applies To | Description |
|-----------|------|------------|-------------|
| `listing_type` | string | all | "product", "service", "property" |
| `service_type` | string | services | "local", "online" |
| `property_transaction_type` | string | property | "rent", "sale" |
| `property_sub_type` | string | property | "building", "house", "apartment", "land" |
| `min_bedrooms` | int | property | Minimum bedrooms (for house/apartment) |
| `min_bathrooms` | int | property | Minimum bathrooms (for house/apartment) |
| `min_area_sqm` | float | property | Minimum area |
| `max_area_sqm` | float | property | Maximum area |

### 3. Update Search Indexing
- Add `listing_type` to search index
- Add property/service-specific fields to index for filtering
- Update `SearchSort` options (maybe "price_per_sqm" for property)

---

## Database Migration Plan

### 1. Add `listing_type` Column
```sql
ALTER TABLE listings ADD COLUMN listing_type VARCHAR(20) DEFAULT 'product';
UPDATE listings SET listing_type = 'product' WHERE listing_type IS NULL;
```

### 2. Create `service_listings` Table (Optional: EAV or JSON)
```sql
CREATE TABLE service_listings (
    listing_id VARCHAR(64) PRIMARY KEY REFERENCES listings(listing_id),
    service_type VARCHAR(20),  -- 'local' or 'online'
    hourly_rate DECIMAL(10,2),
    project_rate DECIMAL(10,2),
    availability JSONB,  -- Store schedule as JSON
    qualifications JSONB,
    service_radius_km INT
);
```

### 3. Create `property_listings` Table
```sql
CREATE TABLE property_listings (
    listing_id VARCHAR(64) PRIMARY KEY REFERENCES listings(listing_id),
    property_transaction_type VARCHAR(10), -- 'rent' or 'sale'
    property_sub_type VARCHAR(20), -- 'building', 'house', 'apartment', 'land'
    area_sqm DECIMAL(10,2),
    bedrooms INT,          -- For house/apartment
    bathrooms INT,         -- For house/apartment
    year_built INT,        -- For building/house/apartment
    lot_size_sqm DECIMAL(10,2), -- For land
    zoning VARCHAR(50)      -- For land
);
```

### 4. Update Search Index
- Modify `search_idx` to include `listing_type`
- Add separate indexes for property/service fields if needed

---

## Frontend/MCP Considerations

### MCP Server (marketplace-mcp)
- Update `create_listing` tool to accept new fields
- Add `listing_type` to MCP tool schemas
- AI agents can now create service/property listings.

### Mobile Apps (Future)
- Android/iOS apps need UI for:
  - Selecting listing type (product/service/property)
  - Showing different forms per type
  - Property: rent vs sale toggle, sub-type selector (building/house/apartment/land)
  - Service: local/online toggle, availability calendar

---

## Search & Performance

### Current Search Implementation
- Uses Moka cache for serialized JSON
- SQL-based filtering in `repositories/listings.rs`
- No full-text search (Phase B feature)

### Impact of New Categories
1. **Search Index**: Need to add `listing_type` to index
2. **Filter Logic**: Update `ListingSearchParams` to include new filters
3. **Cache Invalidation**: Invalidate cache when new listing types are added
4. **Performance**: Should still hit 42k+ ops/s if indexes are proper

### Search Query Example (Updated)
```sql
SELECT * FROM listings l
LEFT JOIN property_listings p ON l.listing_id = p.listing_id
LEFT JOIN service_listings s ON l.listing_id = s.listing_id
WHERE l.listing_type = 'property'
  AND p.property_sub_type = 'apartment'
  AND p.bedrooms >= 2
ORDER BY l.price ASC
LIMIT 20;
```

---

## Phased Implementation Plan

### Phase 1: Backend Data Model ✅ (Planning)
- [ ] Update `api-contract` crate with new types
- [ ] Add `ListingType`, `ServiceType`, `PropertyTransactionType`, `PropertySubType` enums
- [ ] Update `Listing` struct to include conditional fields

### Phase 2: Database Migrations
- [ ] Migration: Add `listing_type` to `listings` table
- [ ] Migration: Create `service_listings` table
- [ ] Migration: Create `property_listings` table
- [ ] Update search index

### Phase 3: API Updates
- [ ] Update `CreateListingRequest` to handle new types
- [ ] Update `SearchRequest` with new filters
- [ ] Update `ListingSummary` / `ListingDetail` responses
- [ ] Add validation (e.g., if `listing_type=service`, require `service_type`)

### Phase 4: Repository & Service Layers
- [ ] Update `ListingRepository` to handle new tables
- [ ] Update `SearchService` to include new filters
- [ ] Update cache invalidation logic

### Phase 5: MCP Server
- [ ] Update `marketplace-mcp` tool schemas
- [ ] Test creating service/property listings via MCP

### Phase 6: Documentation
- [ ] Update OpenAPI spec (`docs/specs/openapi.yaml`)
- [ ] Update `docs/whitepaper/` with new categories
- [ ] Update `docs/server/README.md`

---

## Open Questions for Review

1. **Database Design**:
   - Should we use **single table** with JSONB for variable fields, or **separate tables** (service_listings, property_listings)?
   - *Current proposal*: Separate tables for clarity and indexing.

2. **Search Implementation**:
   - Should we use **full-text search** (Phase B) before adding categories, or add categories first?
   - *Recommendation*: Add categories now, FTS can be layered on later.

3. **Pricing Models**:
   - Services: Should we support **tiered pricing** (different rates for different service types)?
   - Property: Should we support **price per sqm** as a sort option?

4. **Geolocation**:
   - Property: Should we require **exact coordinates** (lat/lng) for properties, or keep city-level for now?
   - *Current*: Keep city-level (already have `location` field).

5. **Images**:
   - Current system has **no image support** (per spec).
   - Should we add image support for property listings (critical for real estate)?
   - *Recommendation*: Defer to Phase B (image support), but plan for it.

---

## Next Steps After Review

1. **Review this document** (team discussion)
2. **Approve/modify** the proposed data model
3. **Prioritize phases** (which to implement first?)
4. **Create GitHub issues** for each phase
5. **Start Phase 1** (update `api-contract` crate)

---

**Document Status**: 📋 DRAFT - Ready for Review  
**Last Updated**: 2026-05-08  
**Author**: AI Assistant (based on user request to expand marketplace categories)
