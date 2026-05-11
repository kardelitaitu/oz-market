# Test Improvement Task Checklist

## Priority 1: Domain Logic Test Suite (Most Critical)

### 1.1 Create domain test directory structure
- [x] Create `backend/server/src/domain/tests/` directory with `mod.rs`
- [x] Add `mod tests;` to `backend/server/src/domain/mod.rs`
- [x] Create sub-module files: `listings.rs`, `negotiation.rs`, `permissions.rs`
- [x] Add `serde_json` to dev-dependencies if needed for test support

### 1.2 Implement test data builders for domain objects
- [x] Define `TestListingBuilder` struct with builder pattern methods
- [x] Implement `TestUserBuilder` for claims/actor contexts
- [x] Implement `TestNegotiationBuilder` for offer states
- [x] Add default helper factory functions (`make_listing()`, `make_user()`)
- [x] Ensure builders produce valid defaults that can be selectively overridden

### 1.3 Add tests for listing validation rules
- [x] Test price constraints (min > 0, max reasonable bounds)
- [x] Test required field validation (title, category, price, owner_id)
- [x] Test field length constraints (title max length, description max length)
- [x] Test URL validation for picture_urls entries
- [x] Test currency code validation (ISO 4217 compliance)
- [x] Test listing_type-specific validation (product vs service vs property)
- [x] Test validation error message correctness

### 1.4 Add tests for listing status transition logic
- [x] Test valid transitions: Active → Sold, Active → Archived, Active → Draft
- [x] Test invalid transitions: Sold → Active, Archived → Sold
- [x] Test that transitions update version counter
- [x] Test that sold listings cannot be modified
- [x] Test archived listing behavior (read-only, hidden from search)
- [x] Test draft → Active transition triggers appropriate validation

### 1.5 Add tests for negotiation workflow business rules
- [x] Test offer price must be > 0 (in `domain/negotiation.rs`)
- [x] Test offer expiration (past dates rejected, future dates accepted) (in `domain/negotiation.rs`)
- [x] Test counter-offer rules (must be between previous offer and asking price) (in `domain/negotiation.rs`)
- [x] Test acceptance requires active listing status (in `domain/negotiation.rs`)
- [x] Test rejection rules (can only reject open offers) (in `domain/negotiation.rs`)
- [x] Test negotiation cannot be opened on sold/archived listings (in `domain/negotiation.rs`)
- [x] Test concurrent negotiation limits per buyer-seller pair (in `domain/negotiation.rs`)

### 1.6 Add tests for user permission business rules
- [ ] Test seller can CRUD own listings (`domain/tests/permissions.rs` is an empty shell)
- [ ] Test seller cannot modify other sellers' listings
- [ ] Test buyer can search but not create listings
- [ ] Test admin can perform all operations on any listing
- [ ] Test support reviewer has read-only access to all listings
- [ ] Test permission escalation boundary (listing create → negotiate flow)

### 1.7 Create assertion helpers for complex domain object comparisons
- [x] Implement `assert_listing_eq(actual, expected)` ignoring timestamps (in `test_support.rs`)
- [x] Implement `assert_negotiation_state_eq(actual, expected)` for state machines (in `test_support.rs`)
- [x] Define `assert_json_roundtrip<T>(val)` for common serde checks (in `test_support.rs`)
- [x] Share helpers via `backend/server/src/test_support.rs` (done)

---

## Priority 2: Enhanced Search Service Tests

### 2.1 Add boundary value testing for numeric search filters
- [ ] Test price min=0 edge (free listings)
- [ ] Test price max=f64::MAX (no upper bound)
- [ ] Test min > max returns empty results
- [ ] Test area_sqm bounds (0, negative, huge)
- [ ] Test bedrooms/bathrooms bounds (0, negative, unreasonably high)
- [ ] Test seller_rating bounds (0.0, 5.0, negative, >5.0)

### 2.2 Test complex query combinations and edge cases
- [ ] Test all filter types combined (category + price + location + status)
- [ ] Test mutually exclusive filters produce empty results
- [ ] Test no filters returns all listings (up to limit)
- [ ] Test query with special characters, unicode, emoji
- [ ] Test very long query strings (SQL injection patterns, boundary lengths)
- [ ] Test case sensitivity (case-insensitive matching verified)

### 2.3 Add geolocation search edge case tests
- [ ] Test antipodal point calculations
- [ ] Test polar region searches (lat=90, lat=-90)
- [ ] Test zero-radius searches returns only exact coordinate match
- [ ] Test missing coordinates handled gracefully
- [ ] Test geolocation_opt_out listings excluded
- [ ] Test distance sorting with multiple points at same location

### 2.4 Test empty/null value handling in all search fields
- [ ] Test None query returns all results sorted by sort_by
- [ ] Test None category returns results from any category
- [ ] Test None price filter returns results at any price
- [ ] Test None location returns results from any location
- [ ] Test None status returns any status
- [ ] Test None/empty cursor returns first page

### 2.5 Verify performance characteristics of sorting algorithms with large datasets
- [ ] Test sort time with 1000+ listings in memory repo
- [ ] Test sort time with single field set (e.g., identical prices)
- [ ] Test sort stability across repeated calls
- [ ] Benchmark relevance scoring with many query terms
- [ ] Document performance expectations and thresholds

### 2.6 Test search relevance scoring with various term combinations
- [ ] Test single term matching in title (highest weight: +20)
- [ ] Test multi-term queries (sum of individual term scores)
- [ ] Test partial word matching
- [ ] Test stop words handling (empty, common words)
- [ ] Test duplicate terms (no double-counting)
- [ ] Test term matching in attributes score
- [ ] Test no matches returns score=0

### 2.7 Add tests for faceted search filter combinations
- [ ] Test category + condition filter combination
- [ ] Test listing_type + service_type combination
- [ ] Test property_transaction_type + property_sub_type combination
- [ ] Test verified_sellers_only + min_seller_rating combination
- [ ] Test near_me + location city overlap (should prefer geolocation)
- [ ] Test all faceted fields set simultaneously

---

## Priority 3: Repository Error Handling Tests

### 3.1 Test repository handling of database constraint violations
- [ ] Test duplicate listing_id insertion returns Conflict error
- [ ] Test duplicate listing fingerprint (same owner + title) returns Conflict error
- [ ] Test foreign key constraint violation on owner_id
- [ ] Test NOT NULL constraint for required fields
- [ ] Test unique constraint violation on custom fields

### 3.2 Test connection failure scenarios and retry logic
- [ ] Test Postgres repo behavior when pool is empty
- [ ] Test query failure returns Storage error category
- [ ] Test connection timeout returns descriptive error
- [ ] Test retry attempt count and backoff behavior
- [ ] Test circuit breaker pattern if implemented

### 3.3 Test transaction rollback behavior on partial failures
- [ ] Test insert failure does not leave partial data
- [ ] Test update failure reverts to previous state
- [ ] Test nested transaction boundaries
- [ ] Test concurrent transaction isolation

### 3.4 Test timeout handling in repository operations
- [ ] Test query timeout returns error (not hang)
- [ ] Test default timeout values are reasonable
- [ ] Test configurable timeout propagation

### 3.5 Test concurrent access safety in repository implementations
- [x] Test concurrent insertions produce no ID collisions (in `repositories/listings.rs`)
- [x] Test concurrent reads while write is in progress (in `repositories/listings.rs`)
- [ ] Test concurrent status updates on same listing
- [ ] Test in-memory repo handles thread contention

### 3.6 Test error mapping from database to domain errors
- [ ] Verify all sqlx error types map to RepositoryError variants
- [ ] Test error message contains context (table, operation)
- [ ] Test sensitive data is not leaked in error messages
- [ ] Test error kind discrimination (Storage vs Conflict vs NotFound)

### 3.7 Test repository behavior with malformed or unexpected data
- [ ] Test listing_id with invalid format
- [ ] Test extreme values for numeric fields
- [ ] Test malformed JSON in attributes field
- [ ] Test very long string inputs
- [ ] Test missing optional fields

---

## Priority 4: Service Layer Orchestration Tests

### 4.1 Create mock implementations for repository dependencies in service tests
- [x] Extract `MockListingRepository` with configurable responses (in `test_support.rs`)
- [ ] Implement `MockNegotiationRepository` for negotiation service tests
- [ ] Implement `MockAuditEventRepository` for event tests
- [ ] Add expectation/assertion helpers to verify mock interactions
- [ ] Document mock usage patterns

### 4.2 Test listing creation workflow
- [ ] Test create → verify listing exists in search results
- [ ] Test create with idempotency key prevents duplicates
- [ ] Test create triggers audit event
- [ ] Test create with invalid data returns ValidationError
- [ ] Test create updates seller statistics

### 4.3 Test negotiation to completion workflow
- [ ] Test offer submission → seller notification
- [ ] Test counter-offer flow (offer → counter → accept)
- [ ] Test offer rejection → negotiation closed
- [ ] Test negotiation timeout → automatic closure
- [ ] Test concurrent negotiations on same listing

### 4.4 Test authorization boundaries in service methods
- [ ] Test unauthorized user gets AuthzError
- [ ] Test authorized user passes through
- [ ] Test expired token/claims rejected
- [ ] Test missing required claims rejected
- [ ] Test authz failure does not execute business logic

### 4.5 Test event publishing verification in service operations
- [ ] Test listing.created event published on successful creation
- [ ] Test negotiation.offer_submitted event published
- [ ] Test event payload contains all required fields
- [ ] Test event order guarantees (causal ordering)
- [ ] Test outbox pattern idempotency

### 4.6 Test service behavior when dependencies fail
- [ ] Test search service handles repository failure gracefully
- [ ] Test listing creation rolls back on event publishing failure
- [ ] Test negotiation service handles concurrent modification errors
- [ ] Test degraded mode behavior (some deps unavailable)

### 4.7 Test cross-service data consistency
- [ ] Test listing status change reflected in search
- [ ] Test seller rating update reflected in search results
- [ ] Test listing deletion removes from search
- [ ] Test category/attribute changes reflected in faceted search

---

## Priority 5: Property-Based Testing Introduction

### 5.1 Add proptest or quickcheck crate to dev dependencies
- [x] Add `proptest = "1"` to `backend/server/Cargo.toml` under `[dev-dependencies]` (done)
- [ ] Add `proptest-derive` if deriving strategies for domain types
- [ ] Configure proptest settings (cases, timeout) for CI environment
- [ ] Add proptest-specific module in test infrastructure

### 5.2 Implement property tests for search scoring algorithms
- [ ] Property: Adding more matching terms never decreases score (monotonicity)
- [ ] Property: Score is commutative across terms (order doesn't matter)
- [ ] Property: Empty query yields score 0 for any listing
- [ ] Property: Title match (20) > description match (10) always
- [ ] Property: Case-insensitive scoring (same score for "MacBook" and "macbook")

### 5.3 Add property tests for price calculation logic
- [ ] Property: Price is positive for valid inputs
- [ ] Property: Price per sqm decreases as area increases (for fixed price)
- [ ] Property: Currency filtering is exact match (not partial)
- [ ] Property: Price sort Asc gives ascending amounts, Desc gives descending

### 5.4 Create property tests for sorting algorithms
- [ ] Property: Sort result length equals input length (no items lost)
- [ ] Property: Sort is transitive (a < b, b < c ⇒ a < c)
- [ ] Property: Tie-breaking by listing_id is deterministic
- [ ] Property: Price sort respects partial_cmp semantics
- [ ] Property: Rating sort treats None as 0.0

### 5.5 Add property tests for validation functions
- [ ] Property: Valid input validates successfully
- [ ] Property: Invalid input fails (by negative test generation)
- [ ] Property: Field length validation rejects strings exceeding max
- [ ] Property: URL validation rejects malformed URLs, accepts well-formed

### 5.6 Document property-based testing approach for team adoption
- [ ] Write quickstart guide for writing property tests
- [ ] Create example property test file as reference
- [ ] Document common property patterns (monotonicity, idempotence, commutativity)
- [ ] Add CI configuration to run proptests with reduced cases

---

## Priority 6: Test Infrastructure Improvements

### 6.1 Create standardized test data builders for all major domain objects
- [ ] Implement `ListingPayloadBuilder` with sensible defaults
- [ ] Implement `SearchRequestBuilder` with common search scenarios
- [ ] Implement `ClaimsBuilder` for auth context simulation
- [ ] Implement `NegotiationOfferBuilder` for offer scenarios
- [ ] Add builder auto-documentation via doc-tests

### 6.2 Implement assertion helpers for complex objects
- [ ] Create `assert_listing_eq` ignoring auto-generated fields (timestamps, IDs)
- [ ] Create `assert_search_response_eq` comparing items without order
- [ ] Create `assert_error_eq` comparing error kind and message pattern
- [ ] Create `assert_json_roundtrip<T>` with pretty-print diff on failure

### 6.3 Create test doubles/mocks for external dependencies
- [ ] Mock `PaymentService` for checkout flow tests
- [ ] Mock `EmailService` for notification tests
- [ ] Mock `StorageService` for file upload tests
- [ ] Implement mock verification (assert_called, assert_not_called)

### 6.4 Establish shared test fixtures for common scenarios
- [ ] Create fixture functions in `test_support.rs`: `sample_listing()`, `sample_search()`
- [ ] Create preset market states: empty, populated (10 items), large (1000 items)
- [ ] Create preset user contexts: admin, seller, buyer, support_reviewer
- [ ] Create fixture for negotiation scenarios: open, active, completed

### 6.5 Create test utility modules for common operations
- [ ] Implement `time::freeze()` helper for time-dependent tests
- [ ] Implement `id::deterministic_generator()` for predictable IDs
- [ ] Implement `random::sample_data()` for randomized test populations
- [ ] Add cleanup helper `with_temp_db()` for integration tests

### 6.6 Document testing patterns and conventions for new contributors
- [ ] Document naming convention: `test_<scenario>_<expected_behavior>()`
- [ ] Document Given/When/Then pattern with blank line separators
- [ ] Document builder pattern usage for complex test data
- [ ] Document mock usage and verification patterns
- [ ] Create example PR showing model test additions

---

## General Tasks

### G.1 Run test coverage analysis to measure baseline and track progress
- [ ] Install `cargo-llvm-cov` for code coverage measurement
- [ ] Run baseline coverage: `cargo llvm-cov --all-targets --html`
- [ ] Document baseline percentages per crate
- [ ] Configure coverage thresholds in CI
- [ ] Add coverage badge to README

### G.2 Update CI/CD configuration to enforce minimum coverage thresholds
- [ ] Add `cargo-llvm-cov` step to CI pipeline
- [ ] Configure threshold enforcement (total ≥70%, new code ≥80%)
- [ ] Add coverage report as CI artifact
- [ ] Gate PR merges on coverage not decreasing

### G.3 Create testing guidelines document
- [ ] Create `docs/TESTING-GUIDELINES.md` with testing philosophy
- [ ] Document test organization (unit vs integration vs e2e)
- [ ] Document mock strategies per layer
- [ ] Document expected test run times and optimization tips
- [ ] Link to property-based testing doc from Priority 5

### G.4 Schedule regular test maintenance and cleanup sessions
- [ ] Define quarterly test review cadence
- [ ] Create checklist for review (flaky tests, slow tests, coverage gaps)
- [ ] Automate flaky test detection in CI
- [ ] Document test ownership per module

### G.5 Add test performance benchmarks to detect regressions
- [ ] Add `criterion` crate for test benchmarks (optional)
- [ ] Create baseline benchmark for key operations (search, insert, sort)
- [ ] Add benchmark comparison in CI PR comments
- [ ] Alert on >20% regression in test execution time
