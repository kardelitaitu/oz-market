# Baseline: Listing ID Cleanup in Tests

## What I Find

### Current State

Test files currently use type-prefixed IDs:
- `product-123`
- `service-456`
- `property-789`

This pattern appears in:
- Test fixtures
- Mock repository returns
- Test helper functions (`TestListingBuilder`)
- URL assertions

### Evidence

1. **Search pattern:** `rg "product-\d+" --type rust` will find affected locations
2. **Test support:** `test_support.rs` likely has builders generating prefixed IDs
3. **Assertions:** Tests check ID prefix to determine listing type

## What I Claim

Removing type prefixes from test IDs will:
- Align tests with new clean ID format
- Simplify test assertions (use `listing_type` field instead of ID)
- Prevent false failures after spec 0001 implementation

## What Is the Proof

1. **Spec 0001 decision:** Clean IDs (no type prefix) are the standard
2. **Consistency:** Tests should mirror production behavior
3. **Simplicity:** `listing_type` field is explicit, ID prefix parsing is fragile
4. **Isolated risk:** Only test files affected, no production code changes