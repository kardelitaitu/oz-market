# Server Module Layout

## Goal

Keep the Rust server compact, explicit, and scalable.

The server should separate:

- transport
- business rules
- persistence
- auth
- abuse controls

without fragmenting into premature microservices.

## Recommended Workspace Shape

Use one Rust workspace for V1.

Suggested layout:

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
      main.rs
      lib.rs
      app.rs
      http/
        mod.rs
        handlers.rs
        runtime.rs
      config.rs
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
    migrations/
      0001_init.sql
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
| `server/http/` | route registration, request parsing, response mapping |
| `server/auth/` | bearer auth, role checks, ownership checks |
| `server/domain/` | server-side domain orchestration glue |
| `server/services/` | orchestration across domain rules and repositories |
| `server/repositories/` | explicit SQL access through `sqlx` |
| `server/models/` | typed request/response and DB row structs |
| `server/errors/` | machine-readable error mapping |
| `server/observability/` | tracing, metrics, diagnostics |
| `server/background/` | lease expiry and async maintenance jobs |

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

## Background Jobs

Recommended first jobs:

1. reservation lease expiration
2. stale reveal cleanup
3. abuse/anomaly aggregation
