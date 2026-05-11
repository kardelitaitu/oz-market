# Plan: Listing ID Cleanup in Tests

## What Is the Solution

### Changes Required

1. **Update test fixtures**
   - Replace `product-123` with `53432`
   - Replace `service-456` with `68924`
   - Replace `property-789` with `11234`

2. **Update mock repository returns**
   - Use clean IDs in `InMemoryListingRepository` test data

3. **Update test assertions**
   - Check `listing_type` field instead of ID prefix
   - Remove ID parsing logic

4. **Update test builders**
   - `TestListingBuilder` generates clean IDs
   - Remove type prefix from ID generation

### Search Commands

```bash
# Find all type-prefixed IDs
rg "product-\d+" --type rust
rg "service-\d+" --type rust
rg "property-\d+" --type rust
```

### Files to Update

- `backend/server/src/test_support.rs` (builders)
- `backend/server/src/**/*_test.rs` (test files)
- `backend/server/src/**/tests/*.rs` (test modules)

### Verification

1. No type-prefixed IDs remain in test files
2. All tests pass with clean IDs
3. `listing_type` field used for type verification