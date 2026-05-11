# Backend Test Coverage Improvement Plan

## Overview

This document outlines the plan to improve test coverage in the backend/server codebase from ~50-60% to 80%+. Focus on adding unit tests for untested modules, expanding integration tests, and introducing E2E tests for critical paths.

## Current State

- **Existing Tests**: 152 unit tests across ~30 files, strong in domain logic (e.g., negotiation.rs: 28 tests). 2 integration tests (postgres_flows.rs).
- **Coverage Estimate**: 50-60% based on file count and test density.
- **Tools**: Cargo test for running; tarpaulin available for measurement (run `cargo tarpaulin --out Html --output-dir coverage`).

## Coverage Gaps

- **Untested Modules**: ~20-25 files (40-50% of codebase), including HTTP handlers, repositories (negotiations.rs, outbox_events.rs), services, config, models.
- **Test Types Missing**: E2E tests for API flows; limited integration with DB/external deps.
- **Edge Cases**: Error handling, concurrency, external failures under-tested.

## Improvement Plan

### Phase 1: Unit Tests for Core Untested Modules (2-3 weeks)
1. **HTTP Layer** (1 week):
   - Add unit tests for `src/http/handlers.rs` and `src/http/actix_runtime.rs` (mock Actix, test endpoints, errors).
   - Target: 10-15 tests each; cover parsing, routing, responses.

2. **Repository Layer** (1 week):
   - Add tests for `src/repositories/negotiations.rs`, `outbox_events.rs`, `idempotency_keys.rs`, `agent_credentials.rs`.
   - Mock DB; test CRUD, transactions, errors.
   - Target: 5-10 tests per file.

3. **Service Layer** (1 week):
   - Test `src/services/mod.rs`, `outbox_events.rs`, `background/mod.rs`.
   - Mock dependencies; cover logic flows.
   - Target: 200+ total unit tests.

### Phase 2: Integration and Edge Cases (1 week)
- Enable and expand `tests/postgres_flows.rs` (use test DB in CI).
- Add tests for DB interactions, auth flows, concurrent operations.
- Cover error scenarios (network, invalid data).

### Phase 3: E2E and Measurement (1 week)
- Add E2E tests using `actix-web-test`: Full flows (create listing → negotiate → close).
- Run tarpaulin; enforce 80% in CI.
- Refactor for testability (dependency injection).

## Tools and Best Practices

- **Measurement**: Tarpaulin for HTML reports; integrate in CI.
- **Mocking**: Use `mockall` for repos/services.
- **Patterns**: Follow existing (e.g., domain tests); async with tokio.
- **CI**: Add `cargo tarpaulin --fail-under 80` to workflows.

## Success Criteria

- 80%+ coverage measured by tarpaulin.
- All major modules tested (unit), critical flows (integration/E2E).
- Tests pass in CI; no regressions.

## Next Steps

1. Install/enable tarpaulin; measure baseline.
2. Start Phase 1: HTTP layer tests.
3. Commit incrementally; update JOURNAL.md with progress.