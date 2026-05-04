# Performance And Stack

## Target

The backend should stay `compact`, `efficient`, and capable of `50k+ requests per second` on a medium-tier server for the right workload shape.

That target should be treated as:

- realistic for lightweight reads and bridge operations
- possible for simple validated writes with careful design
- unrealistic if every request performs heavy database work or external calls

## Recommended Backend Direction

| Option | Pros | Cons | Recommendation |
| --- | --- | --- | --- |
| Rust | Mature backend ecosystem, strong memory safety, excellent async/runtime support, strong JSON and PostgreSQL tooling | More verbose than Zig | Best |
| Zig | Very fast, low-level control, compact binaries | Weaker backend ecosystem, fewer proven web/DB libraries, higher delivery risk | Experimental only |
| Node.js | Fast delivery and large ecosystem | Weak fit for strict throughput and latency predictability at this target | Weak |

## Recommended Rule

If `50k+ RPS` is a hard requirement and the service must stay maintainable, choose `Rust`.

Use `Zig` only if the goal is low-level experimentation and you accept more delivery and ecosystem risk.

## Hot-Path Rules

- keep the service as one binary
- keep MCP thin and avoid duplicating business logic in the adapter layer
- avoid microservices in V1
- avoid ORM-heavy abstractions
- prefer explicit SQL or a very light query layer
- keep request auth cheap
- return and accept plain JSON over HTTP APIs
- no image uploads, processing, or proxying
- no synchronous third-party calls on request path
- use idempotency keys for retry-heavy endpoints
- push non-critical events to async processing
- keep search on indexed dimensions and avoid unbounded scans

## API Shape Guidance

Prioritize a small API:

- `POST /v1/listings`
- `GET /v1/listings/{id}`
- `GET /v1/listings/search`
- `POST /v1/negotiations`
- `POST /v1/negotiations/{id}/offers`
- `POST /v1/negotiations/{id}/request-contact-reveal`

Avoid feature spread early. Every extra endpoint adds maintenance and testing cost.

## Data Strategy

| Strategy | Pros | Cons |
| --- | --- | --- |
| PostgreSQL with typed columns + JSONB | Best balance of reliability, indexed reads, and flexible API payloads | Needs discipline in schema design |
| PostgreSQL + Redis | Better read offload and token/cache speed | More moving parts |
| Redis-heavy first | Extremely fast reads/writes | Weak primary system of record for negotiation and audit state |

## Recommendation

Start with `PostgreSQL` as the source of truth.

Model it like this:

- typed columns for hot filters and indexed search paths
- `JSONB` for flexible product metadata
- API responses served as plain `JSON`

Move to `PostgreSQL + Redis` only after load testing shows the database is the bottleneck.

For search:

- start with PostgreSQL indexes and full-text/trigram support
- add an external search engine only after measurement proves PostgreSQL is the limit

## Rust Database/API Direction

Recommended Rust stack:

- `axum` or `actix-web` for HTTP
- `serde` for JSON request/response
- `sqlx` for explicit SQL and PostgreSQL access
- `tokio` runtime
- thin MCP adapter over the same core service layer

This stack is compact, proven, and fits the `API or JSON` requirement cleanly.

## Benchmark Definition Needed

Before implementation, define:

- request mix percentage for `read`, `write`, and `search`
- payload sizes
- authentication cost
- acceptable `p95` and `p99` latency
- whether `50k+ RPS` means one node or horizontal cluster total

Without that definition, the target is too vague to design against reliably.
