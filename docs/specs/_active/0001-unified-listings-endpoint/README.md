---
id: 0001-unified-listings-endpoint
title: Unified /api/listings/{id} Endpoint
status: proposed
owner: backend-team
implementer: pending
priority: P2
area:
  - backend
  - api
files:
  code:
    - backend/server/src/handlers/listings.rs
    - backend/crates/api-contract/src/lib.rs
    - backend/crates/api-contract/src/endpoints.rs
  docs:
    - docs/specs/openapi.yaml
    - docs/DOCS-README.md
    - docs/whitepaper/10-api-contract.md
acceptance:
  - GET /api/listings/{id} returns any listing type
  - Response includes listing_type field
  - Old endpoints return 301 redirect with Deprecation header
  - 4-week gradual deprecation completes
non_goals:
  - Database schema changes
  - Search index changes
  - Other CRUD endpoint changes
risks:
  - External consumers may break (mitigated by redirect)
  - Mobile client update required
---

# Unified /api/listings/{id} Endpoint

Status: `proposed`

Owner: `backend-team`
Implementer: `pending`

## Summary

Replace three separate type-specific endpoints with a single unified listing retrieval endpoint. Consolidate `GET /api/product/{id}`, `GET /api/service/{id}`, `GET /api/property/{id}` into `GET /api/listings/{id}`.

## Scope

### In Scope
- Backend handler merge
- API contract update
- OpenAPI spec update
- Mobile client updates
- Old endpoint deprecation (301 redirect, 4 weeks)

### Out of Scope
- Database schema changes
- Search index changes
- Other CRUD endpoints

## Decisions

| Decision | Value |
|----------|-------|
| Old URLs | 301 redirect to `/api/listings/{id}` |
| ID format | `"id": "123"` (clean, no type prefix) |
| Cutover | Gradual (4-week deprecation) |

### Deprecation Timeline

- **Week 1:** New endpoint live. Old endpoints return `Deprecation` header.
- **Week 4:** Old endpoints return 301 redirect.
- **Month 2:** Remove old endpoint handlers.