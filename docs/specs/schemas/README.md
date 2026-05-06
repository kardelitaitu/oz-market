# Shared Schema Component Definitions

This directory contains documentation for reusable schema components used in the Project Marketplace API.

## Overview

The API contract is defined using a **spec-first** approach:
- **Source of truth**: `../openapi.yaml` (frozen spec)
- **Rust types**: `backend/crates/api-contract/src/` (implements the spec)
- **Validation**: `utoipa` ToSchema derives (compile-time OpenAPI compatibility)

---

## Component Schema Map

### Type Aliases (String Patterns)

| OpenAPI Component | Rust Type Alias | Pattern/Constraint | utoipa Annotation |
|-----------------|-------------------|---------------------|----------------------|
| `ResourceId` | `ResourceId` (String) | 1-128 chars | `#[schema(value_type = String)]` |
| `CurrencyCode` | `CurrencyCode` (String) | `^[A-Z]{3}$` | `#[schema(value_type = String)]` |
| `CountryCode` | `CountryCode` (String) | `^[A-Z]{2}$` | `#[schema(value_type = String)]` |

**Validation Functions** (in `api-contract/src/listing.rs`):
- `validate_resource_id(id)` → checks 1-128 char length
- `validate_currency_code(code)` → checks 3-letter uppercase pattern
- `validate_country_code(code)` → checks 2-letter uppercase pattern

---

### Enum Types

| OpenAPI Component | Rust Enum | Derives | Serde/Renaming |
|-----------------|-----------|----------|----------------|
| `Category` | `Category` | `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema` | `#[serde(rename_all = "snake_case")]` `#[schema(rename_all = "snake_case")]` |
| `Condition` | `Condition` | Same as above | Same as above |
| `ListingStatus` | `ListingStatus` | Same as above | Same as above |
| `SearchSort` | `SearchSort` | Same as above | Same as above |
| `NegotiationStatus` | `NegotiationStatus` | Same as above | Same as above |
| `ContactRevealStatus` | `ContactRevealStatus` | Same as above | Same as above |
| `ApiErrorCode` | `ApiErrorCode` | Same as above | Same as above |

**Values Mapping** (all use `snake_case` in both YAML and Rust):

| Component | Values |
|-----------|--------|
| `Category` | laptop, phone, tablet, desktop, monitor, accessory, camera, audio, gaming, appliance, furniture, vehicle_part, other |
| `Condition` | new, used, refurbished |
| `ListingStatus` | draft, active, reserved, sold, archived |
| `SearchSort` | relevance, newest, price_asc, price_desc |
| `NegotiationStatus` | open, countered, near_close, reserved, contact_requested, contact_revealed, closed, cancelled |
| `ContactRevealStatus` | pending, approved, rejected, expired |
| `ApiErrorCode` | invalid_field, missing_field, conflict, not_found, rate_limited, unauthorized, forbidden, owner_mismatch, credential_revoked, quota_exceeded, trust_review_required, reservation_conflict, version_conflict, invalid_transition |

---

### Struct Types

#### Listing Service

| OpenAPI Component | Rust Struct | Key Fields | utoipa Annotations |
|-------------------|-------------|------------|---------------------|
| `Price` | `Price` | `currency: CurrencyCode`, `amount: f64` | `#[schema(value_type = String)]` on `currency` |
| `ListingLocation` | `ListingLocation` | `country_code: CountryCode`, `country_name: String`, `city: String` | `#[schema(value_type = String)]` on `country_code` |
| `ListingPayload` | `ListingPayload` | `schema_version`, `owner_id`, `category`, `product_name`, `condition`, `price`, `location`, `picture_urls`, `description`, `attributes` | `#[schema(value_type = String)]` on `owner_id`, `schema_version`; `#[schema(value_type = Object)]` on `attributes` |
| `CreateListingRequest` | `CreateListingRequest` | `idempotency_key: String`, `listing: ListingPayload` | `#[schema(value_type = String)]` on `idempotency_key` |
| `ListingSummary` | `ListingSummary` | `listing_id: ResourceId`, `status: ListingStatus`, `version: u64`, `listing: ListingPayload` | `#[schema(value_type = String)]` on `listing_id` |
| `CreateListingResponse` | `CreateListingResponse` | `listing_id: ResourceId`, `status: ListingStatus`, `version: u64`, `created_at: String` | `#[schema(value_type = String)]` on `listing_id`; `#[schema(format = DateTime)]` on `created_at` |
| `SearchPriceFilter` | `SearchPriceFilter` | `currency: Option<CurrencyCode>`, `min_amount: Option<f64>`, `max_amount: Option<f64>` | `#[schema(value_type = String)]` on `currency` |
| `SearchLocationFilter` | `SearchLocationFilter` | `country_code: Option<CountryCode>`, `city: Option<String>` | `#[schema(value_type = String)]` on `country_code` |
| `SearchRequest` | `SearchRequest` | `query`, `category`, `condition`, `price`, `location`, `status`, `sort_by`, `limit`, `cursor` | `#[schema(value_type = String)]` on `query`, `cursor` |
| `SearchResponse` | `SearchResponse` | `items: Vec<ListingSummary>`, `applied_sort_by: SearchSort`, `next_cursor: Option<String>` | `#[schema(value_type = String)]` on `next_cursor` |

#### Negotiation Service

| OpenAPI Component | Rust Struct | Key Fields | utoipa Annotations |
|-------------------|-------------|------------|---------------------|
| `OpenNegotiationRequest` | `OpenNegotiationRequest` | `listing_id`, `buyer_agent_id`, `offer_currency`, `offer_amount`, `idempotency_key` | `#[schema(value_type = String)]` on all String fields |
| `SubmitOfferRequest` | `SubmitOfferRequest` | `offer_currency`, `offer_amount`, `idempotency_key` | `#[schema(value_type = String)]` on currency and key |
| `RequestContactRevealRequest` | `RequestContactRevealRequest` | `idempotency_key` | `#[schema(value_type = String)]` on key |
| `NegotiationResponse` | `NegotiationResponse` | `negotiation_id`, `listing_id`, `buyer_agent_id`, `status`, `offer_currency`, `latest_offer_amount`, `reservation_lease_id`, `final_offer_amount`, `version`, `updated_at` | `#[schema(value_type = String)]` on ID fields and timestamps |
| `ContactRevealResponse` | `ContactRevealResponse` | `reveal_id`, `negotiation_id`, `reveal_status`, `revealed_phone_reference`, `expires_at`, `approved_at`, `updated_at` | `#[schema(value_type = String)]` on ID fields and timestamps |

#### Error Types

| OpenAPI Component | Rust Struct | Key Fields | utoipa Annotations |
|-------------------|-------------|------------|---------------------|
| `ApiErrorDetail` | `ApiErrorDetail` | `code: ApiErrorCode`, `message: String`, `field: Option<String>` | `#[schema(value_type = String)]` on `message`, `field` |
| `ApiErrorResponse` | `ApiErrorResponse` | `error: ApiErrorDetail` | (wraps `ApiErrorDetail`) |

---

## File Locations

### OpenAPI Spec (Frozen)
- **File**: `../openapi.yaml`
- **Section**: `components/schemas`

### Rust Types (with `utoipa` ToSchema)
- **Enums & Structs**: `backend/crates/api-contract/src/listing.rs` (11 types)
- **Negotiation Types**: `backend/crates/api-contract/src/negotiation.rs` (7 types)
- **Error Types**: `backend/crates/api-contract/src/error.rs` (3 types)

### Validation Helpers
- **Functions**: `backend/crates/api-contract/src/listing.rs`
  - `validate_resource_id()`
  - `validate_currency_code()`
  - `validate_country_code()`

---

## `utoipa` ToSchema Integration

### What We Did (Session 2026-05-05 --21:09)
1. Added `utoipa = "5"` to `api-contract/Cargo.toml`
2. Added `#[derive(ToSchema)]` to all 21 types
3. Added `#[schema(value_type = String)]` to type alias fields
4. Added `#[schema(format = DateTime)]` to timestamp fields
5. Added `#[schema(value_type = Object)]` to `serde_json::Value` fields

### Compile-Time Benefits
- Types now carry OpenAPI schema metadata
- Future: Can generate spec from code using `utoipa-gen`
- CI: Can validate code changes don't break OpenAPI contract

### Current Policy
- **Frozen spec**: The YAML remains the source of truth
- **utoipa usage**: Type safety and future generation capability only
- **No regeneration**: We do NOT regenerate the spec from code (would violate frozen policy)

---

## Cross-Reference: OpenAPI YAML → Rust

### How to Verify Alignment

1. **Check enum values**: Compare YAML `enum:` lists with Rust enum variants
2. **Check field names**: Ensure Rust field names match YAML `properties:` (using `snake_case`)
3. **Verify types**: Ensure `value_type = String` annotations match YAML `type: string` with patterns
4. **Run tests**: `cd backend && cargo test` (all 47 tests must pass)

### Example: `ListingStatus`

**YAML**:
```yaml
  ListingStatus:
    type: string
    enum: [draft, active, reserved, sold, archived]
    example: active
```

**Rust**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(rename_all = "snake_case")]
pub enum ListingStatus {
    Draft,
    Active,
    Reserved,
    Sold,
    Archived,
}
```

---

## Next Steps (from `../README.md`)

This document fulfills item #1 from `docs/specs/README.md`:
- ✅ Shared schema component definitions (this document)
- ⬜ Generated client/server contract notes
- ⬜ Internal `/internal/v1` spec when needed
- ⬜ CI workflow file when automation starts
- ⬜ Any `/internal/v1` policy docs when needed

---

## Maintenance Notes

- When adding new API types: Add to both `openapi.yaml` AND `api-contract` with `ToSchema`
- When modifying existing types: Update both YAML and Rust code
- Run `cargo test` after any schema changes
- Keep the `utoipa` annotations in sync with YAML constraints (patterns, min/max lengths, etc.)
