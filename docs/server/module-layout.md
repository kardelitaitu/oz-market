# Server Module Layout

> last audited 06-06-26 by docs-auditor

## Goal

Keep the Rust server compact, explicit, and scalable.

The server should separate:

- transport
- business rules
- persistence
- auth
- abuse controls

without fragmenting into premature microservices.

## Workspace Shape

```text
backend/
  Cargo.toml
  crates/
    api-contract/
      src/
        lib.rs
        listing.rs
        negotiation.rs
        error.rs
    auth-core/
      src/
        lib.rs
  server/
    src/
      bootstrap.rs
      bin/
        bootstrap_schema.rs
        check_db.rs
        populate_db.rs
        update_coordinates.rs
        pg_search_bench.rs
        http_bench.rs
        sse_bench.rs
        bench_concurrent.rs
        phase5_bench.rs
        schema_test.rs
        bench_suite.rs
      main.rs
      lib.rs
      app.rs
      openapi.rs
      test_support.rs
      config.rs
      http/
        mod.rs
        handlers.rs
        util.rs
        actix_handlers.rs
        actix_runtime.rs
      models/
        mod.rs
        db.rs
      repositories/
        mod.rs
        listings.rs
        negotiations.rs
        reservations.rs
        contact_reveals.rs
        audit_events.rs
        outbox_events.rs
        seller_accounts.rs
        agent_credentials.rs
        ledger.rs
        reviews.rs
        idempotency_keys.rs
      domain/
        mod.rs
        ledger.rs
        listing_validation.rs
      services/
        mod.rs
        agent.rs
        agent_dispatcher.rs
        agent_metrics.rs
        agent_registry.rs
        agent_routing.rs
        async_committer.rs
        audit_events.rs
        authz.rs
        circuit_breaker.rs
        contact_reveals.rs
        idempotency.rs
        latency_scorer.rs
        ledger_cache.rs
        outbox_events.rs
        rate_limiter.rs
        reservations.rs
        search.rs
        wal.rs
      errors/
        mod.rs
      observability/
        mod.rs
      background/
        mod.rs
      bench/
        mod.rs
        driver.rs
        scheduler.rs
        resource_monitor.rs
        report.rs
        distributed.rs
        drivers/
          mod.rs
          http.rs
          postgres.rs
          sse.rs
          wal.rs
          cache.rs
    migrations/
      0001_init.sql
      0002_add_seller_quota_fields.sql
      0003_fix_reservation_leases.sql
      0004_add_marketplace_fields.sql
      0005_add_seller_display_name_and_rating.sql
      0006_create_reviews_table.sql
      0007_add_coordinates.sql
      0008_add_listing_type.sql
      0009_create_service_listings.sql
      0010_create_property_listings.sql
      0011_create_user_accounts.sql
      0012_add_search_indexes.sql
      0013_add_negotiation_offer_history.sql
      0014_add_credit_ledger.sql
  mcp/
    src/
      main.rs
      lib.rs
```


## Module Responsibilities

| Module | Responsibility |
| --- | --- |
| `api-contract/` | shared request/response and error schemas |
| `auth-core/` | claims, roles, scopes, ownership checks |
| `server/http/` | route registration, request parsing, response mapping, actix runtime |
| `server/auth/` | bearer auth, role checks, ownership checks |
| `server/domain/` | validation rules (listing, ledger) and permission logic |
| `server/services/` | orchestration across domain rules and repositories (agent dispatch, circuit breaker, rate limiter, search, async committer, WAL, ledger cache) |
| `server/repositories/` | explicit SQL access through `sqlx` (listings, negotiations, reservations, reviews, ledger, idempotency keys, contact reveals, audit events, outbox, seller accounts, agent credentials) |
| `server/models/` | typed request/response and DB row structs |
| `server/errors/` | machine-readable error mapping |
| `server/observability/` | tracing, metrics, diagnostics |
| `server/background/` | reserved for lease expiry and async maintenance jobs (not yet implemented) |
| `server/bench/` | benchmark framework (drivers, scheduler, resource monitor, distributed gRPC protocol, report/CI gating) |
| `server/openapi.rs` | OpenAPI schema generation |
| `server/test_support.rs` | shared test helpers |

## Dependency Direction

Recommended direction:

- `server -> api-contract + auth-core`
- `http -> services`
- `services -> domain + repositories + auth`
- `repositories -> models/db`
- `domain` should not depend on `http`
- `api-contract` and `auth-core` should stay transport-agnostic

## Shared Service Rule

Both HTTP and MCP must call the same service functions:

- `create_listing`
- `get_listing`
- `search_listings`
- `open_negotiation`
- `submit_offer`
- `request_contact_reveal`
- `approve_contact_reveal`

## Persistence Rule

Use explicit SQL with `sqlx`.

Keep hot-path tables focused:

- `listings`
- `negotiations`
- `reservation_leases`
- `contact_reveals`
- `audit_events`
- `seller_accounts`
- `agent_credentials`
- `credit_ledger`
- `idempotency_keys`

## Background Jobs

Not yet implemented. `background/mod.rs` is a placeholder. Recommended first jobs:

1. reservation lease expiration
2. stale reveal cleanup
3. abuse/anomaly aggregation
