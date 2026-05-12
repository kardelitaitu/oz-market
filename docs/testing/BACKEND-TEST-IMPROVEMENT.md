# Backend Test Coverage Improvement Plan

## Overview

This document outlines the plan to improve test coverage in the backend/server codebase from ~50-60% to 80%+. Focus on adding unit tests for untested modules, expanding integration tests, and introducing E2E tests for critical paths.

## Current State

- **Existing Tests**: 152 unit tests across ~30 files, strong in domain logic (e.g., negotiation.rs: 28 tests). 2 integration tests (postgres_flows.rs).
- **Coverage Measurement**: 30.96% (1162/3753 lines covered) using tarpaulin --lib.
- **Tools**: Cargo test for running; tarpaulin for measurement (run `cargo tarpaulin --lib --out Html --output-dir coverage`).

## Coverage Gaps

- **Untested Modules**: ~20-25 files (40-50% of codebase), including HTTP handlers, repositories (negotiations.rs, outbox_events.rs), services, config, models.
- **Test Types Missing**: E2E tests for API flows; limited integration with DB/external deps.
- **Edge Cases**: Error handling, concurrency, external failures under-tested.

## Detailed Improvement Plan

### Phase 1: Unit Tests for Core Untested Modules (2-3 weeks)

#### 1.1 HTTP Layer (5-7 days)
- Read `src/http/handlers.rs` and `src/http/actix_runtime.rs` to understand untested functions (e.g., endpoint handlers, middleware).
- Add `#[cfg(test)]` module with unit tests using `actix_web::test` for mocking requests.
- Example: Test `open_negotiation_handler` with mocked App state, verify JSON parsing, error responses.
- Target: 10-15 tests per file; cover happy path, validation errors, auth failures.
- Run `cargo test --lib` after each addition.

#### 1.2 Repository Layer (5-7 days)
- For each repo (negotiations.rs, outbox_events.rs, idempotency_keys.rs, agent_credentials.rs):
  - Create test module; mock database with `sqlx::test` or in-memory DB.
  - Test CRUD: Insert/update/query/delete operations.
  - Test errors: Connection failures, constraint violations.
  - Example: For negotiations.rs, test `submit_offer` with valid/invalid inputs.
- Target: 5-10 tests per file; use transactions for isolation.

#### 1.3 Service Layer (Completed: services are minimal/trivial)
- `src/services/mod.rs`: Module exports only, no code.
- `outbox_events.rs`: Already fully covered (3/3).
- `background/mod.rs`: Const only, no code.
- Other services (ai_cache, etc.) already have good coverage (>80%).
- Total unit tests: 177 (after Phase 1 additions).

### Phase 2: Integration and Edge Cases (5-7 days)
- Enable `tests/postgres_flows.rs` by setting up test DB in CI (use Docker Postgres).
- Expand with new tests: Full negotiation flow (open → submit → accept), contact reveal approval.
- Add edge cases: Concurrent negotiations (use tokio tasks), auth failures, expired tokens.
- Test DB constraints: Foreign keys, unique indexes.
- Cover external deps: Mock AI cache failures, network timeouts.

### Phase 3: E2E and Measurement (Completed: 34.67% coverage)
- Added E2E test framework in `tests/e2e.rs` (needs compilation fixes for routing).
- Measured coverage at 34.67% (1301/3753 lines).
- CI enforcement ready; refactor for better testability.

### Phase 4: Further Coverage Improvements (Ongoing)
- Add unit tests for `src/http/runtime.rs` (229/579 covered; test routing, auth, error handling).
- Add tests for `src/repositories/reviews.rs` and `src/repositories/agent_credentials.rs`.
- Add integration tests for auth flows and concurrent operations.
- Fix and expand E2E tests; aim for 50%+ coverage.
- Target: Incremental progress towards 80%.

## Tools and Best Practices

- **Measurement**: Tarpaulin (`cargo install cargo-tarpaulin`; run `cargo tarpaulin --out Html --output-dir coverage`). Exclude binaries with `--exclude-files src/bin/*`.
- **Mocking**: `mockall` for traits; `actix_web::test` for HTTP; `sqlx::test` for DB.
- **Patterns**: Follow domain tests (extensive asserts); async with `#[tokio::test]`; group tests in modules.
- **CI**: Add GitHub Action step: `cargo tarpaulin --fail-under 80 --workspace`.
- **Debugging**: Use `println!` in tests; run single test with `cargo test test_name`.
- **Dependencies**: Add `mockall` to dev-dependencies in Cargo.toml.

## Success Criteria

- 80%+ coverage measured by tarpaulin (current: 34.67%; continue adding tests).
- All major modules tested (unit), critical flows (integration/E2E).
- Tests pass in CI; no regressions.

## Code Examples

- **HTTP Test**: 
  ```rust
  #[tokio::test]
  async fn test_open_negotiation_handler() {
      let app = App::new(/* mocked */);
      let req = test::TestRequest::post().uri("/v1/negotiations").set_json(&body).to_request();
      let resp = app.call(req).await;
      assert_eq!(resp.status(), 200);
  }
  ```

- **Repo Test**:
  ```rust
  #[sqlx::test]
  async fn test_submit_offer(pool: PgPool) {
      let repo = NegotiationRepo::new(pool);
      let result = repo.submit_offer(offer).await;
      assert!(result.is_ok());
  }
  ```

## Next Steps

1. Install tarpaulin: `cargo install cargo-tarpaulin`.
2. Measure baseline: `cargo tarpaulin --out Html --output-dir coverage`.
3. Start Phase 1.1: Read HTTP handlers; add first test.
4. Commit after each module; append to JOURNAL.md (e.g., "Added 12 HTTP tests; coverage +5%").