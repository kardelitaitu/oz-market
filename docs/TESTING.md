# Testing Guide

## Quick Start

```bash
# Run all lib tests (unit + integration, ~0.15s)
cd backend/server && cargo test --lib

# Run with all targets (includes doctests, binaries)
cargo test --all-targets

# Run with coverage
cargo llvm-cov --lib --html
# open target/llvm-cov/html/index.html

# Clippy
cargo clippy --all-targets -- -D warnings
```

## Test Organization

Tests live next to the code they test (inline `#[cfg(test)] mod tests`), not in a separate `tests/` directory.

| Module | Layer | Coverage | Tests |
| --- | --- | --- | --- |
| `domain/listing_validation.rs` | Validation | 99.55% | 33 |
| `domain/negotiation.rs` | Business rules | 90.88% | ~30 |
| `domain/status_transitions.rs` | State machine | 93.49% | 20 |
| `services/search.rs` | Search service | 98.52% | ~70 |
| `services/contact_reveals.rs` | Contact service | 87.76% | 2 |
| `services/idempotency.rs` | Idempotency | 83.80% | ~10 |
| `repositories/listings.rs` | In-memory repo | 64.23% | ~25 |
| `repositories/seller_accounts.rs` | In-memory repo | 11.50% | + |
| `repositories/contact_reveals.rs` | In-memory repo | 37.39% | + |
| `app.rs` | App orchestration | 90.57% | ~30 |
| `http/actix_handlers.rs` | Actix transport | 20.54% | 8 |
| `http/runtime.rs` | TCP runtime | 55.83% | 6 |

## Naming Convention

Tests use snake_case with the pattern `test_<scenario>_<expected_behavior>`:

```rust
#[tokio::test]
async fn test_create_listing_duplicate_returns_conflict() {
    // Given
    // When
    // Then
}
```

## Data Builders

Use `crate::test_support::*` helpers:

```rust
use crate::test_support::*;

let listing = TestListingBuilder::default()
    .title("MacBook Pro")
    .price(2500.0)
    .build();

let seller = seller_claims();
let buyer = buyer_claims();
let admin = admin_claims();
```

Available assertion helpers:

- `assert_listing_eq(a, b)` — compares ignoring auto-generated fields
- `assert_negotiation_state_eq(a, b)` — compares ignoring timestamps/IDs
- `assert_json_roundtrip(val)` — serde roundtrip check

## Running Specific Tests

```bash
# By test name
cargo test test_search_title_match

# By module path
cargo test services::search::tests

# Property-based tests (behind proptest feature)
cargo test services::search::tests::proptests
```

## Coverage Philosophy

- **Target**: overall ≥70% line coverage
- **Included**: all in-memory repository code, domain logic, services, app orchestration, HTTP handler helpers
- **Excluded (require Postgres)**: `repositories/*.rs` Postgres impl blocks, `models/db.rs`, `openapi.rs`
- **Goal**: every error path and every business rule branch has a test

## Coverage Workflow

```bash
# Summary
cargo llvm-cov --lib --summary-only

# Per-file HTML report
cargo llvm-cov --lib --html
```

## Adding Tests Checklist

1. Find the right module — tests go inline in `#[cfg(test)] mod tests`
2. Use `InMemory*Repository` for persistence — never mock repos unless testing service error handling
3. Use `Test*Builder` from `test_support` for test data
4. Cover both happy path and error path
5. Verify: `cargo test --lib && cargo clippy --all-targets`
