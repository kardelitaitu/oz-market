---
id: 0002-listing-id-cleanup
title: Clean Test File IDs (Remove Type Prefixes)
status: completed
owner: backend-team
implementer: opencode
priority: P2
area:
  - backend
  - testing
files:
  code:
    - backend/server/src/**/*_test.rs
    - backend/server/src/**/tests/*.rs
    - backend/server/src/test_support.rs
acceptance:
  - All test files use clean IDs (no type prefix)
  - Test assertions verify listing_type field
  - Test builders generate clean IDs
non_goals:
  - Production code ID changes
  - Database schema changes
risks:
  - None (isolated to test files)
---

# Clean Test File IDs

Status: `proposed`

Owner: `backend-team`
Implementer: `pending`

## Summary

Update all test files to use clean listing IDs instead of type-prefixed IDs (`product-123` → `53432`).

## Scope

### In Scope
- All backend test files
- Test support utilities (builders, fixtures)

### Out of Scope
- Production code
- Database schema

## Decisions

| Decision | Value |
|----------|-------|
| ID format | Clean (no type prefix) |
| Type verification | Use `listing_type` field in assertions |