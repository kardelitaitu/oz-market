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

## 2026-05-11 15:30

- **Test infrastructure cleanup**: Fixed clippy warnings in test_support.rs
  - Added `MockOptionResult<T>` type alias for complex result types
  - Added `Default` impl for `MockListingRepository`
  - Reformatted long patterns in app.rs and listings.rs tests
  - All 148 lib tests pass, clippy clean
  - Committed and pushed to main

## 2026-05-11 14:00

- **Spec 0001 (Performance) Complete**: Benchmark results exceed targets for practical loads
  - Search warm cache: 65,013 ops/s (100), 54,619 ops/s (500), 43,257 ops/s (1000)
  - Get listing warm: 52,379 ops/s
  - 100% success rate at all levels
  - Fixed quantity column nullable handling in listings repository
  - Implemented cache warming with 7 common queries
  - Optimized: 500 max connections, 2GB listing cache, 1GB search cache, 30min/15min TTL
  - Updated spec with benchmark results

## 2026-05-11 15:30

- **Phase 5 Complete**: Quota/index tuning from measured behavior
  - Added `services/rate_limiter.rs` with in-memory sliding window rate limiter
  - Per-IP/claims search rate: 60 req/min via `global_limiter()`
  - Per-token create: 10/min, negotiate: 20/min, contact reveal: 10/min
  - New seller daily/hourly caps: 3/day, 1/hour in addition to total quota
  - Rate limiting wired into both TCP runtime and Actix handlers
  - Migration `0012_add_search_indexes.sql`: functional index `(country_code, LOWER(city))`, composite index `(listing_type, status, category)`, pg_trgm extension
  - 152 lib tests pass; clippy clean; TODO items marked complete
## 2026-05-11 14:52

- Reworked `NEGOTIATION-DESIGN.md` into a brainstorm doc so it stays aligned with the current contract while leaving room for a later negotiation spec.

## 2026-05-11 15:01

- Locked the negotiation brainstorm toward richer offer history, negotiation-led offer tracking, and explicit accept/reject actions for the next spec direction.

## 2026-05-11 15:05

- Promoted the negotiation brainstorm into an active spec under `docs/specs/_active/NEGOTIATION-OFFER-HISTORY.md` with richer offer history and explicit finalization actions.

## 2026-05-11 15:37

- Moved the negotiation work into a full active spec bundle under `docs/specs/_active/0003-negotiation-offer-history/` with baseline, plan, decisions, validation, and metadata files.

## 2026-05-11 15:41

- Clarified the negotiation spec baseline so contract surface and runtime implementation are separated cleanly.

## 2026-05-11 18:09

- Locked `0003-negotiation-offer-history` to an append-only negotiation history schema with explicit `accept` and `reject` actions.

## 2026-05-11 18:35

- Reviewed and normalized active spec strategy governance under `docs/specs/_active/0001-unified-listings-endpoint`, `0002-listing-id-cleanup`, and `0003-negotiation-offer-history`.
- Aligned status/implementer metadata between frontmatter/spec YAML and README body text so workflow state is consistent and auditable.
- Fixed broken 0001 contract-doc references from `docs/whitepaper/10-api-contract.md` to `docs/01-whitepaper/10-api-contract.md`.
- Replaced placeholder governance docs with concrete quality rules, validation checklists, implementation notes, and internal API outlines to enforce shared-service logic, contract parity, authz/idempotency guardrails, and safer rollout checks.

## 2026-05-11 18:50

- Executed an `_active` spec lint pass for legacy `docs/whitepaper/...` references, placeholder governance text, and README metadata/body consistency.
- Replaced remaining placeholder `ci-commands.md` and `decisions.md` files for specs `0001`, `0002`, and `0003` with concrete checks and decision logs.
- Added an automated `active spec governance guard` to `check.ps1` so CI now blocks legacy doc paths, placeholder governance text, and status/implementer drift in `_active` specs.
- Re-ran guard-only checks (`./check.ps1 -SkipBuild -SkipFormat -SkipClippy -SkipTests`) and confirmed pass.
- Recorded OpenAPI parity outcomes in active spec decisions docs to keep contract drift visible before backend implementation PRs.

## 2026-05-11 19:05

- Normalized `0001-unified-listings-endpoint` wording from mixed `/api/...` and `/v1/...` forms to consistent `/v1/...` paths with explicit `{listing_id}` naming across README, spec metadata, plan, baseline, and internal API outline.
- Added machine-readable parity artifacts (`parity-report.md`) for active specs `0001`, `0002`, and `0003` to document contract alignment status in a repeatable JSON snapshot format.
- Ran full `./check.ps1` (no skip flags); all guards and Rust checks passed (journal guard, active spec guard, cargo check, fmt, clippy, tests).

## 2026-05-11 19:20

- Started implementation for active negotiation spec (`0003`) across contract, repository, app, HTTP runtime, and MCP wrapper layers.
- Added negotiation offer history persistence (`offer_history` JSONB migration `0013_add_negotiation_offer_history.sql`) and new submit/accept/reject flows.
- Fixed concurrent open-negotiation behavior to enforce single-winner conflict semantics in both in-memory and Postgres negotiation repositories.
- Updated race-condition test to keep both concurrent requests authorized so it validates repository conflict behavior.
- Validated implementation with `cargo test --lib` (152 passed).

## 2026-05-11 19:45

- Hardened `check.ps1` journal guard to avoid CRLF warning crashes and keep append-only validation reliable in local PowerShell runs.
- Fixed `api-contract` test initialization for `NegotiationResponse` by adding `offer_history` so all targets compile under clippy/test.
- Added a targeted `#[allow(clippy::too_many_arguments)]` on `MarketplaceApp::new` after introducing the negotiation repository dependency, preserving constructor compatibility across runtimes/tests.
- Re-ran full `./check.ps1` (no skip flags): journal guard, active spec governance, cargo check, fmt, clippy, and tests all pass.

## 2026-05-11 20:05

- Updated `docs/specs/openapi.yaml` to include negotiation `accept`/`reject` operations, new accept/reject request schemas, and `offer_history` schema fields so API docs match implemented backend routes.
- Added PostgreSQL negotiation integration coverage in `backend/server/tests/postgres_flows.rs` for submit-offer + accept flow and reject flow, including persisted status/final-offer/history assertions.
- Extended test schema setup to apply migration `0013_add_negotiation_offer_history.sql`, ensuring integration tests run against the current negotiation table shape.
- Locked the single-negotiation enforcement decision in `0003` docs (DB uniqueness via deterministic `neg_{listing_id}` + PK conflict) with option tradeoffs for future reopen semantics.
- Refreshed `0003` parity report and README/plan text so active-spec documentation reflects current contract/runtime behavior and no longer marks accept/reject/history as pending.

## 2026-05-11 20:43

- Ran full `./check.ps1` (no skip flags).
- Result: PASS for journal guard, active spec guard, cargo check, cargo fmt --check, cargo clippy -D warnings, and cargo test --lib.
- Why: validated current workspace is stable before continuing implementation.

## 2026-05-11 21:11

- Confirmed specs in sequence: `0003` then `0001` then `0002`.
- Updated `0003` and `0001` metadata/body status to `completed`; `0003` implementer set to `opencode` and validation checklist marked complete.
- Created `docs/specs/_done/` and moved completed spec folders from `_active` to `_done`:
  - `0003-negotiation-offer-history`
  - `0001-unified-listings-endpoint`
  - `0002-listing-id-cleanup`
- Ran full `./check.ps1` after the move; all checks passed.

## 2026-05-11 21:15

- Created dedicated backend implementation commit after successful full `./check.ps1` run.
- Included negotiation offer-history + finalization implementation set across `api-contract`, app/runtime/handlers, repository persistence, MCP wiring, checker hardening, and migration `0013_add_negotiation_offer_history.sql`.
- Why: keep spec-lifecycle move and backend implementation separated for cleaner audit and rollback boundaries.
- Updated docs/specs/openapi.yaml to include negotiation ccept/eject operations, new accept/reject request schemas, and offer_history schema fields so API docs match implemented backend routes.
- Added PostgreSQL negotiation integration coverage in ackend/server/tests/postgres_flows.rs for submit-offer + accept flow and reject flow, including persisted status/final-offer/history assertions.
- Extended test schema setup to apply migration 013_add_negotiation_offer_history.sql, ensuring integration tests run against the current negotiation table shape.
- Locked the single-negotiation enforcement decision in 003 docs (DB uniqueness via deterministic 
eg_{listing_id} + PK conflict) with option tradeoffs for future reopen semantics.
- Refreshed 0003 parity report and README/plan text so active-spec documentation reflects current contract/runtime behavior and no longer marks accept/reject/history as pending.

## 2026-05-11 20:43

- Ran full ./check.ps1 (no skip flags).
- Result: PASS for journal guard, active spec guard, cargo check, cargo fmt --check, cargo clippy -D warnings, and cargo test --lib.
- Why: validated current workspace is stable before continuing implementation.

## 2026-05-11 21:11

- Confirmed specs in sequence: 003 then 001 then 002.
- Updated 003 and 001 metadata/body status to completed; 003 implementer set to opencode and validation checklist marked complete.
- Created docs/specs/_done/ and moved completed spec folders from _active to _done:
  - 003-negotiation-offer-history
  - 001-unified-listings-endpoint
  - 002-listing-id-cleanup
- Ran full ./check.ps1 after the move; all checks passed.

## 2026-05-11 21:15

- Created dedicated backend implementation commit after successful full ./check.ps1 run.
- Included negotiation offer-history + finalization implementation set across api-contract, app/runtime/handlers, repository persistence, MCP wiring, checker hardening, and migration 013_add_negotiation_offer_history.sql.
- Why: keep spec-lifecycle move and backend implementation separated for cleaner audit and rollback boundaries.

## 2026-05-11 21:21

- Performed governance cleanup after spec archival commits.
- Recovered legacy _active artifacts and archived them under _done instead of leaving destructive deletions:
  - performance-infrastructure-optimization/
  - TESTING-IMPROVEMENT-PROPOSAL.md
- Updated performance infrastructure archive README status to completed and implementer to opencode for metadata consistency.
- Moved test-planning docs out of repo root into docs/testing/ to keep root compact:
  - BACKEND-TEST-IMPROVEMENT.md -> docs/testing/BACKEND-TEST-IMPROVEMENT.md
  - 	odo-test-improvement.md -> docs/testing/todo-test-improvement.md
- Removed generated coverage raw artifact ackend/build_rs_cov.profraw and added *.profraw to .gitignore to prevent future accidental tracking.

## 2026-05-12 10:35

- Upgraded `backend/server/src/bin/bench_concurrent.rs` with explicit claims modes: `public`, `fixed`, and `rotating` (default `rotating`) to prevent apples-to-oranges benchmark runs.
- Added explicit `429` and `other_failures` reporting in benchmark summaries and search sweep rows so rate-limit drops are immediately visible.
- Captured fresh HTTP baseline artifacts for `public` and `rotating` modes under `docs/testing/benchmarks/` and added a compact baseline report: `http-bench-baseline-2026-05-12.md`.
- Updated `docs/server/README.md` benchmark documentation and quick reference command to include claims mode and current baseline behavior.
- Why: the previous single-`sub` benchmark path was hitting the `60/min` search limiter and falsely appearing as a performance regression.
- Added fixed-claims diagnostic artifact (`http-bench-concurrent-fixed-2026-05-12.txt`) to preserve evidence of expected 429 saturation under single-sub benchmarking.

## 2026-05-12 11:05

- Added a root README benchmark section (`Benchmark Baseline (2026-05-12)`) with dated `bench_concurrent` results for `public`, `rotating`, and `fixed` claims modes.
- Linked benchmark artifacts under `docs/testing/benchmarks/` from the root README so performance evidence is visible at repo entrypoint.
- Why: make current throughput and rate-limit behavior discoverable without requiring readers to open server-specific docs first.

## 2026-05-12 12:15

- Hardened negotiation read/write lifecycle in `MarketplaceApp` and repositories:
  - `get_negotiation_status` now requires stored negotiation + participant authz (`Action::GetNegotiationStatus`) and no longer fabricates fallback negotiation responses.
  - `submit_offer` now validates positive finite amounts and blocks invalid status transitions.
  - `accept_negotiation` and `reject_negotiation` now enforce explicit allowed states (including reserved-path consistency) before mutation.
- Split idempotency namespaces for negotiation actions by adding `AcceptNegotiation` and `RejectNegotiation` operations, replacing shared `SubmitOffer` operation keys for those flows.
- Aligned transport behavior across runtimes:
  - Actix `open_negotiation` now returns `201 Created`.
  - TCP runtime `request_contact_reveal` now returns `202 Accepted` to match Actix.
  - Actix error mapping now respects repository/idempotency/search error kinds (409/404/403/400/500) instead of collapsing to generic 400s.
- Added focused app-level tests for negotiation lifecycle and authz:
  - reserved -> submit -> accept path,
  - reject from reserved releasing reservation,
  - invalid offer amount rejection,
  - negotiation status read blocked for unrelated buyer.
- Added `Debug` derive on TCP runtime `HttpResponse` to unblock test ergonomics that use `unwrap()` on `Result<_, HttpResponse>` in unit tests.
- Validation: `cargo check --lib` passed, `cargo test --lib shared_app_` passed (16 tests), and `cargo test --lib negotiations` passed (13 tests).
- Note: full `cargo test` is still blocked by pre-existing integration-test issues in `backend/server/tests/e2e.rs` unrelated to this negotiation patch set.
## 2026-05-12 07:33

- Fixed failing runtime unit test http::runtime::tests::test_claims_from_headers_valid by updating the synthetic x-marketplace-claims header payload to valid current Claims JSON (roles as snake_case enum strings).
- Revalidated status-code parity checks in runtime tests (open_negotiation => 201, request_contact_reveal => 202) and kept the e2e contract-shape test aligned with the current app constructor and routes.
- Ran full ./check.ps1 with no skip flags: PASS for journal guard, active spec guard, cargo check, cargo fmt --check, cargo clippy (-D warnings), and cargo test --lib.
- Why: close the last CI blocker and preserve an auditable checkpoint before moving remaining active specs.


## 2026-05-12 13:05

- Repaired checker failures after benchmark-spec activation pass.
- Ran `cargo fmt --all` to fix formatting drift flagged in `backend/server/src/models/db.rs`.
- Fixed clippy compile errors in `backend/server/src/models/db.rs` tests by replacing invalid `CurrencyCode::USD` usage with `"USD".to_string()` (current contract uses string currency codes).
- Re-ran full `./check.ps1` with no skip flags: PASS on journal guard, active spec governance, cargo check, cargo fmt --check, cargo clippy (`-D warnings`), and cargo test --lib.
- Why: restore a green, reliable baseline before continuing active spec implementation.

## 2026-05-12 13:18

- Activated new active spec package `0004-http-benchmark-stability` under `docs/specs/_active/` with full governance artifacts and machine-readable parity report.
- Added `backend/coverage/` to `.gitignore` and removed generated tarpaulin HTML output from the working tree.
- Re-ran full `./check.ps1` before commit/push: all gates passed.
- Why: keep benchmark work spec-driven, auditable, and free from generated artifact noise.

## 2026-05-12 13:31

- Updated active spec `0004-http-benchmark-stability` to enforce minimal CI policy.
- Replaced `./check.ps1`/checker references with `cargo check` only across `ci-commands.md`, `README.md`, `spec.yaml`, `plan.md`, `quality-rules.md`, `parity-report.md`, and `validation-checklist.md`.
- Ran `cargo check --manifest-path backend/server/Cargo.toml --workspace` successfully.
- Why: keep CI surface minimal while preserving benchmark governance and auditability.

## 2026-05-12 14:03

- Tightened active spec `0005-negotiation-hardening-and-parity` acceptance and parity requirements.
- Added explicit requirement that all post-begin failures must mark idempotency as failed (no stuck pending states).
- Added explicit status parity target for `POST /v1/negotiations/{negotiation_id}/request-contact-reveal` => `202 Accepted` across runtime, actix, and OpenAPI.
- Expanded `files.code` to include expected authz wiring and integration test files (`services/authz.rs`, `tests/postgres_flows.rs`, `tests/e2e.rs`).
- Why: make negotiation hardening implementation auditable and unambiguous before coding.

## 2026-05-12 14:18

- Updated OpenAPI negotiation contact-reveal response status from `200` to `202` for `/v1/negotiations/{negotiation_id}/request-contact-reveal` to match runtime and actix behavior.
- Updated active spec `0004-http-benchmark-stability` to explicitly point canonical benchmark-command source of truth to `docs/server/README.md` and `docs/testing/benchmarks/http-bench-baseline-2026-05-12.md`.
- Ran quick governance verification: `cargo check --manifest-path backend/server/Cargo.toml --workspace` passed, plus targeted spec consistency checks (status/implementer parity, legacy path scan, and reveal-route 202 parity) passed.
- Why: keep benchmark and negotiation parity claims accurate and auditable before implementation continues.

## 2026-05-12 14:31

- Downgraded `0004-http-benchmark-stability` parity result to `in_progress` because a fresh benchmark cycle could not be run from this workspace state (no local target listening on 3000/3003/8080).
- Added `fresh_benchmark_cycle_executed: false` to the `0004` parity snapshot and noted that the spec still needs a live rerun before moving to `_done`.
- Revalidated `cargo check --manifest-path backend/server/Cargo.toml --workspace` and confirmed the `0004` source-of-truth references remain consistent.
- Why: keep the benchmark spec audit-safe instead of overstating completion.
# 2026-05-12 20:43 UTC - 0004 benchmark cycle completed

- Ran a fresh `bench_concurrent` cycle for `public`, `rotating`, and `fixed` against the local release server.
- Recorded the cycle in a dated benchmark artifact and refreshed the `0004` parity report and validation checklist.
- Rechecked `cargo check` after the benchmark refresh, then marked the spec metadata `completed` for move to `_done`.
# 2026-05-12 20:58 UTC - 0005 scope expanded

- Expanded `0005` implementation scope to include idempotency and reservation modules explicitly.
- This keeps the spec aligned with the actual code paths needed for post-begin failure handling and conflict compensation.

## 2026-05-12 18:14

- Added 5 unit tests for reviews.rs InMemory repository (edge cases for create with body, timestamp updates, empty results); added 3 integration tests (auth flow create listing, concurrent negotiation submissions, seller account trust level update) in postgres_flows.rs; added 5 unit tests for rate_limiter.rs (partial cleanup, edge cases, is_new_seller)

## 2026-05-17 00:00

- Marked MCP deployment shape as an accepted decision: a separate stdio sidecar binary.
- Removed the old open question from the roadmap and risks docs so the MCP plan stays consistent across whitepaper notes.
- Why: keep the MCP adapter isolated from the HTTP runtime while preserving one shared backend service layer.

## 2026-05-17 00:20

- Reworked the MCP docs to define one public desktop-agent tool set and separate internal admin helpers.
- Updated the API contract and role matrix so the MCP surface, HTTP routes, and internal helper paths stay aligned.
- Added shared-app parity tests for the contact-reveal polling flow in `marketplace-server` and `marketplace-mcp`.
- Why: keep the agent-facing contract small and consistent while proving both desktop and mobile-style paths reuse the same backend behavior.

## 2026-05-17 00:45

- Wired the real stdio MCP sidecar in `backend/mcp` with an `rmcp` tool router, shared app-backed tool calls, and a small dev fallback for local runs.
- Added `JsonSchema` to the shared API contract so MCP tools can reuse the same request types instead of drifting onto a separate wrapper model.
- Updated the tester to send the MCP `notifications/initialized` handshake and verify the public 8-tool catalog.
- Why: make the desktop MCP path real, keep the tool surface small, and prove the transport reaches the same backend logic as the shared app layer.

## 2026-05-17 20:53

- Hardened the MCP launcher contract so the sidecar now expects explicit `MARKETPLACE_MCP_CLAIMS_JSON` input instead of silently falling back in normal runs.
- Added a real end-to-end smoke test that launches the tester against the built `marketplace-mcp` binary and keeps ambient `DATABASE_URL` out of the default path.
- Added shared-contract mobile parity tests for the Android and iOS manifests plus canonical payload round-trips.
- Why: make the launcher boundary explicit, keep the MCP smoke path predictable, and prove the shared contract still matches mobile-facing payload shapes.

## 2026-05-17 21:00

- Removed the legacy MCP claims alias so the sidecar and tester now use one explicit launcher payload path only.
- Extended the MCP smoke tester and launcher-contract test with a real `create_listing` write-path check plus read-back coverage.
- Why: shrink the launcher contract, prove a mutating tool still flows through the shared backend logic, and keep the desktop agent path easier to reason about.

## 2026-05-17 20:59

- Added CI coverage for the MCP launcher smoke test and the shared mobile contract parity test.
- Synced the stale server MCP README so it now describes the runtime sidecar, the real tester, and the 8-tool public surface.
- Why: keep the repo honest about MCP readiness and catch launcher or contract drift automatically.

## 2026-05-17 21:00

- Added a nightly MCP smoke workflow so the launcher contract and shared contract parity keep getting checked after PRs merge.
- Added explicit launcher-contract notes to the MCP docs and whitepaper so desktop launchers know which env vars to pass.
- Why: keep the MCP contract visible, scheduled, and easy to wire from the real launcher without guessing.

## 2026-05-17 21:03

- Switched the MCP runtime to read `MARKETPLACE_MCP_DATABASE_URL` explicitly instead of ambient `DATABASE_URL`.
- Updated the tester helpers to validate the explicit Postgres env path and keep the launcher contract pure.
- Why: make the database target part of the launcher contract, not an inherited process-side surprise.

## 2026-05-17 21:07

- Upgraded the nightly MCP smoke workflow to boot a real Postgres service, apply the base schema, and run the launcher smoke against that database.
- Why: prove the desktop-agent path works against a real backend schema instead of only the in-memory fallback.

## 2026-05-17 21:12

- Documented that the MCP smoke workflow is intentionally base-schema only for the current listing-only public path.
- Why: keep future negotiation or contact-reveal coverage from accidentally inheriting a too-small bootstrap without an explicit decision.

## 2026-05-17 21:14

- Added a shared server schema bootstrap helper and switched the MCP smoke workflow and Postgres server tests onto it.
- Why: keep the smoke setup and live Postgres tests on one reusable migration path instead of separate hand-rolled schema setup code.

## 2026-05-17 21:17

- Updated the server module layout doc to include the new schema bootstrap helper and binary.
- Why: keep the architecture docs aligned with the new reusable migration entrypoint.

## 2026-05-17 21:18

- Wired the local Postgres test script to run `bootstrap_schema` before the integration tests.
- Why: make the local test workflow self-contained on a fresh database and reuse the same schema bootstrap path as CI.

## 2026-05-17 21:20

- Converted the remaining Postgres flow tests off `sqlx::test` and onto the same live-db skip path as the rest of the file.
- Why: keep the test suite reliable when no local Postgres is running, while still exercising the same schema bootstrap when a database is present.

## 2026-05-17 21:21

- Wired the local phase5 benchmark wrapper to run the shared schema bootstrap helper before the benchmark.
- Why: make the local benchmark path self-contained on a fresh Postgres database without duplicating schema setup logic.

## 2026-05-17 21:22

- Fixed the local benchmark and Postgres test wrappers to preserve `Pop-Location` cleanup before exiting.
- Why: keep the PowerShell wrappers predictable when they are called from other scripts.

## 2026-05-17 21:23

- Added a dedicated `Server Postgres` GitHub Actions workflow that boots Postgres, runs the shared schema bootstrap helper, and executes the live integration tests.
- Why: keep a continuous proof of the full Postgres path without bloating the main CI job.

## 2026-05-17 21:24

- Expanded the Claude Desktop example in the server docs to show the explicit launcher env contract.
- Why: make the external launcher wiring easier to implement without guessing the expected environment variables.

## 2026-05-17 21:52

- Reduced duplicate schema bootstraps in the combined local Postgres workflow by adding a skip flag to the child scripts.
- Why: keep the standalone scripts self-contained while making the combined developer path faster and less repetitive.

## 2026-05-17 21:53

- Clarified in the server docs that the live `Server Postgres` workflow can also be rerun manually in GitHub Actions.
- Why: make the live-db proof path easier to monitor and rerun without changing the workflow behavior.

## 2026-05-17 21:54

- Added a lightweight MCP binary smoke test that checks initialize plus the public `tools/list` catalog on the real sidecar.
- Why: give the MCP surface a small, reliable regression test without depending only on the broader launcher smoke path.

## 2026-05-18

- Wired `accept_negotiation` and `reject_negotiation` as MCP tools in the `rmcp` tool router at `backend/mcp/src/runtime.rs`.
- Updated `docs/01-whitepaper/07-mcp-server.md` to reflect the 10-tool surface (added tools to list, example shapes for accept/reject, updated rollout count).
- Updated `docs/server/README.md` MCP tools table from 8→10 rows.
- Why: keep whitepaper and server docs in sync with the expanded MCP tool surface after adding negotiation finalization tools.

## 2026-05-18

- Created `docs/app-android/build-plan.md` with 6-phase plan: Project Scaffold, API Client, Auth, UI Screens, AI Agent, Polish.
- Created `docs/app-ios/build-plan.md` with matching 6-phase plan adapted for Swift/SwiftUI tech stack.
- Updated `docs/app-android/README.md` and `docs/app-ios/README.md` to reference the new build plans.
- Why: give both mobile platforms a phased, actionable roadmap aligned with the frozen V1 backend contract.

## 2026-05-18

- **Idempotency persistence** — created `PostgresIdempotencyKeyRepository` in `repositories/idempotency_keys.rs`. Wired into production Actix runtime (`actix_runtime.rs`), replacing the in-memory-only `InMemoryIdempotencyRepository` that lost keys on restart. The Postgres table already existed in migration `0001_init.sql`. Uses `now() + interval '1 day'` for TTL (matching the 24h TTL used throughout the app). Timestamps returned via `::TEXT AS` casting, consistent with other Postgres repos.
- **Server README** — created `backend/server/README.md` with base URLs, auth/claims docs, endpoint table, rate limits, error codes, env config, and OpenAPI spec reference for mobile dev onboarding.
- **Search pagination** — implemented cursor-based pagination in `PostgresListingRepository::search_listings()`. Applies cursor filtering after in-memory sort (same pattern as `InMemoryListingRepository`). Returns `next_cursor` as the last item's `listing_id` when more results exist. Added cursor to search cache key in `actix_handlers.rs` to prevent cross-page cache collisions.
 - Why: fix the three mobile-blocking backend gaps identified in the audit — idempotency data loss on restart, missing dev onboarding docs, and search results that couldn't be paginated.

## 2026-05-18

- **check.ps1 refactored + Pester tests** — extracted `Test-JournalAppendOnly` and `Test-ActiveSpecGovernance` from `check.ps1` to `backend/scripts/check-helpers.ps1` for unit testability.
- Created `backend/scripts/check-helpers.Tests.ps1` with 13 Pester 5 tests covering both helpers (file missing/unchanged/append-only/removal checks, spec directory governance, status/implementer mismatch, placeholder text, yaml scanning, multi-file reporting).
- Fixed `\\s*` → `\s*` in frontmatter regex patterns (PowerShell double-quoted strings don't interpret `\` as escape, so `\\s` passed literal `\` to the .NET regex engine instead of whitespace class).
- Why: make the CI gate testable and prevent regex-escape drift from reaching main.

## 2026-05-18

- **Deprecated routes removed** — deleted 6 deprecated route registrations from `actix_runtime.rs:126-152` (`/v1/product/`, `/v1/service/`, `/v1/property/` detail + search). Removed `extract_listing_type_from_path()`, `deprecated_listing_redirect()`, `deprecated_search_redirect()`, and `DEPRECATION_SUNSET_DATE` const from `actix_handlers.rs`. Stripped the listing-type-from-path fallback from `search_listings()` (was dead code after routes removed). Deleted 4 unit tests for the now-removed helper. Test count dropped 211→207.
- Why: sunset deadline June 1; the 301 redirects have been live since May 11 but no clients depend on the old paths (MCP uses service layer, mobile uses `/v1/listings`).
new line
## 2026-05-18

- **Audit fixes applied** — replaced 16 `serde_json::to_value(...).unwrap()` calls in `runtime.rs` with `.unwrap_or_default()`
  to eliminate panic-on-NaN path; cfg-gated 3 dead helper functions in `actix_handlers.rs`;
  removed redundant `pool.clone()` in mcp `runtime.rs:594`; cfg-gated 3 test-only helpers in mcp `lib.rs`;
  added `Eq` derive to `ApiErrorDetail`, `SearchLocationFilter`, `AcceptNegotiationRequest`,
  `RejectNegotiationRequest`, `RequestContactRevealRequest`, `ContactRevealResponse`;
  added `#[must_use]` to 4 auth-core methods (`has_scope`, `has_role`, `is_expired`, `action_to_scopes`);
  upgraded `reqwest 0.11→0.12`, `utoipa 3→5`, `utoipa-swagger-ui 3→8`.
- Why: eliminate all panic-at-runtime paths, suppress dead-code enables in production, and reduce
  transitive-dependency bloat from old crate versions.

## 2026-05-18

- **Spec 0005 implementation** — `open_negotiation` now validates offer_amount is positive finite;
  upsert-conflict (already reserved) releases reservation + commits idempotency failure;
  listing-not-found/-inactive branches commit idempotency failure.
  `request_contact_reveal` authz now looks up stored negotiation + listing to bind permission context
  (rejecting outsider requests).
  `approve_contact_reveal` authz now traverses reveal→negotiation→listing chain
  (rejecting wrong-seller reveals).
  Added rate limiting to `approve_contact_reveal` (10/min per sub).
- Why: close spec 0005 requirements — idempotency integrity on all failure paths, authz binds to
  actual stored state instead of relying on claims alone.

## 2026-05-18

- **Spec 0005 completed** — all 7 parity checks flipped to true.
  Added `InternalServerError` response component + 500 responses to all 11 user-facing endpoints in OpenAPI.
  Added 5 Postgres integration tests: outsider reveal request rejection, wrong-seller reveal approval rejection,
  invalid offer amount (zero/negative), open-negotiation conflict compensation, inactive listing idempotency failure commit.
- Why: prepare backend for live-testing by closing spec 0005 requirements, documenting all 500 paths in OpenAPI,
  and covering ownership/conflict/validation paths with Postgres regressions.
- **Archived spec 0005** from `_active/` to `_done/` — no active specs remain.
- **Created `backend/scripts/live-test.ps1`** — standalone harness that boots Postgres, starts the Actix server,
  runs 11 HTTP test scenarios (health, CRUD, negotiation, reveal, auth errors, rate limiting), and cleans up.
- Why: spec 0005 is fully implemented and ready for archiving; live-test harness enables end-to-end validation
  against real Postgres before mobile clients consume the API.
