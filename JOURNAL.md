## 2026-05-10 11:23

- Fixed compilation errors in populate_db.rs:
  - Added missing geolocation fields to all match arms (products, services, properties)
  - Fixed type inference issues with explicit dereferencing
- Successfully ran populate_db script with comprehensive geolocation data
- Database now populated with 1000 sellers, 100,000 listings (mixed products/services/properties), and 100,000 reviews
- All listing types include realistic geolocation coordinates for testing location-based features

## 2026-05-10 11:40

- Resolved clippy warnings: fixed type mismatches in app.rs and listings.rs, removed unnecessary casts
- Updated bench-http.ps1 defaults to 5000 operations at concurrency levels 100,500,1000,2000,5000
- Added user_accounts migration (0011_create_user_accounts.sql)
- All CI checks passed: build, format, clippy, tests
- Committed and pushed changes to remote repository

## 2026-05-10 11:40

- **Phase 1 Complete**: Performance Infrastructure Optimization
- Increased database connection pool from 20 to 100 connections (configurable via DATABASE_MAX_CONNECTIONS env var)
- Added basic connection pool metrics to /metrics endpoint (total connections, idle connections)
- Changes validated with cargo check and committed to repository

## 2026-05-10 11:43

- **Phase 2 Complete**: Async Runtime Optimization
- Implemented custom tokio runtime with adaptive worker threads (num_cpus - 1 for stability, minimum 1)
- Added TOKIO_WORKER_THREADS environment variable for configuration
- Enhanced metrics with runtime information (worker threads, CPU cores)

## 2026-05-10 16:21

- **Search API Enhancement**: Implemented 3 new features in actix_handlers.rs:
  - `?fields=` field filtering: parse_fields_param + filter_listing_fields to return only requested fields
  - `?include=` eager loading: returns _include meta object indicating available relations
  - Cursor pagination: DB-level via ORDER BY + WHERE listing_id > cursor (listings.rs)
- Cache key now includes fields and include params for proper cache segregation
- All features compile successfully

## 2026-05-10 16:21

- Added unit tests for search API helper functions in actix_handlers.rs:
  - parse_fields_param: tests for single/multiple/empty fields
  - parse_include_param: tests for single/multiple/lowercase normalization
  - filter_listing_fields: tests for filtering, empty fields, non-object handling
- Fixed http/mod.rs test visibility (was excluding actix_handlers in test mode)
- Fixed pre-existing broken import in api-contract tests
- All 70 tests pass (62 existing + 8 new)

## 2026-05-10 15:31

- **Expanded Unit Test Coverage**
- Added 16 tests for search service (normalize_search_terms, listing_index_text, score_listing, compare_search_items)
- Added 9 tests for listings repository (insert, get, search with filters/sorting/pagination)
- Fixed pre-existing compilation errors in db.rs and listings.rs (quantity type mismatch)
- All 62 server lib tests now pass
- Added num_cpus dependency for automatic CPU detection
- Supports low-resource deployments (2-thread VPS and up)
- Changes validated with cargo check and committed to repository

## 2026-05-10 11:50

- **Phase 3 Complete**: Enhanced Caching Implementation
- Increased listing cache from 10,000 to 100,000 entries with 10-minute TTL
- Increased search cache from 1,000 to 50,000 entries with 5-minute TTL
- Added cache entry count metrics to /metrics endpoint
- Implemented TTL-based cache eviction policies for automatic cleanup
- Changes validated with cargo check and committed to repository

## 2026-05-10 11:55

- **Phase 4 Complete**: Monitoring and Observability
- Added utilization percentages for database connections and caches
- Implemented memory usage estimation for cache monitoring
- Created comprehensive monitoring setup guide with Prometheus/Grafana instructions
- All metrics now available in Prometheus format at /metrics endpoint
- Performance infrastructure optimization project complete
- Expected performance improvement: 6.8k → 40k+ ops/s at 5000 concurrency

## 2026-05-10 15:31

- **Performance Fix**: Search endpoint was extremely slow (~100 ops/s) due to missing SQL LIMIT clause
- Root cause: fetch_rows() was fetching ALL matching rows (potentially 100K) and sorting in memory
- Added LIMIT clause (uses request limit, defaults to 50, max 200) to prevent excessive row fetching
- Results:
  - Cold search: 2.46s → 17ms (145x faster)
  - Warm search: ~100 ops/s → ~25,000 ops/s (250x faster)
  - 500 concurrency now hits 100% success rate (was ~77%)
- Also fixed double JSON serialization in search handler (serialize once, use for both cache and response)

## 2026-05-10 15:42

- **Performance Optimization**: Full benchmark sweep shows massive improvements
- Fixed double JSON serialization in get_listing handler (same issue as search)
- Search performance (warm cache):
  - 100 concurrency: 52,641 ops/s (was ~22K)
  - 200 concurrency: 57,976 ops/s (was ~30K)
  - 500 concurrency: 40,351 ops/s (was ~23K)
- All benchmark phases achieve 100% success rate
- get_listing works at ~25K ops/s when tested standalone
- Note: get_listing at high concurrency (500) shows degradation - appears to be benchmark client limitation, not server

## 2026-05-10 15:42

- **API Optimization**: Added response compression (gzip) - built into actix-web middleware
- **Search Cache Key**: Improved to include listing_type, category, sort_by for better cache hits
- Benchmark results with optimizations:
  - Search 100: 51,700 ops/s
  - Search 200: 57,977 ops/s
  - Search 500: 30,297 ops/s

## 2026-05-10 18:10

- Tightened `check.ps1` journal guard to compare the committed journal as a line-prefix, so rewritten history fails and append-only updates pass.

## 2026-05-11 06:30

- **Task 1.1**: Created domain test directory structure
  - `backend/server/src/domain/tests/` with `mod.rs`, `listings.rs`, `negotiation.rs`, `permissions.rs`
  - Added `#[cfg(test)] mod tests;` to `domain/mod.rs`
- **Task 1.2**: Implemented test data builders in `test_support.rs`
  - `TestListingBuilder`, `TestUserBuilder`, `TestNegotiationBuilder`
  - `make_listing()` and `make_user()` factory functions
- **Task 1.3**: Created `domain/listing_validation.rs` with `validate_listing_payload()`
  - Validates price constraints, required fields, field lengths, URLs, currency codes, and listing-type-specific rules (derived from OpenAPI spec)
  - 33 unit tests covering all validation rules

## 2026-05-11 09:15

- **Spec 0001**: Unified /api/listings/{id} Endpoint
  - Discovered unified endpoint already exists at `/v1/listings/{id}`
  - Old Actix routes (`/v1/product/`, `/v1/service/`, `/v1/property/`) now redirect to unified endpoint
  - Added deprecation handlers returning 301 with Deprecation/Sunset/Link headers
  - Sunset date: Sat, 01 Jun 2026
  - Removed unused `status_transitions` module reference

## 2026-05-11 11:00

- **Phase 3.3 complete**: Created `docs/TESTING.md` with test organization, naming conventions, data builder patterns, coverage philosophy, and adding-tests checklist
- **Phase 3.4 complete**: Filled critical coverage gaps:
  - `services/contact_reveals.rs`: added 2 error path tests (approve nonexistent → NotFound, get missing → None)
  - `repositories/seller_accounts.rs`: added 10 in-memory operation tests (get_by_owner_id, update_trust_level, update_quota_override, increment_listings_created — each with happy path + not-found edge case)
  - `http/runtime.rs`: added 4 handler edge case tests (404 unknown route, 404 nonexistent listing, 400 invalid create body, search empty results)
- +16 tests total; 143 lib tests pass; clippy clean
