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
    marketplace-core/
      src/
        lib.rs
  server/
    src/
      main.rs
      lib.rs
      app.rs
      config.rs
      http/
        mod.rs
      auth/
        mod.rs
      domain/
        mod.rs
      services/
        mod.rs
      repositories/
        mod.rs
      models/
        mod.rs
      errors/
        mod.rs
      observability/
        mod.rs
      background/
        mod.rs
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
| `marketplace-core/` | core business rules and state transitions |
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

- `server -> api-contract + auth-core + marketplace-core`
- `http -> services`
- `services -> domain + repositories + auth`
- `repositories -> models/db`
- `domain` should not depend on `http`
- `api-contract`, `auth-core`, and `marketplace-core` should stay transport-agnostic

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
