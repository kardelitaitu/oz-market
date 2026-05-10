# Architecture Options

## Decision Target

Choose the smallest architecture that fits an `agent-first marketplace bridge`, a `50k+ RPS` target, and the intended product surfaces.

## Option Comparison

| Option | Pros | Cons | Fit |
| --- | --- | --- | --- |
| Server + MCP + mobile apps | Matches the intended product, supports both agents and users, keeps website optional | More client integration work than server-only | Best |
| Single API bridge service only | Smallest codebase, direct agent integration, best chance to hit throughput goal | Incomplete against the intended end product | Acceptable as first internal milestone |
| Website + backend from day one | Human friendly immediately | More moving parts, lower focus on hot-path optimization | Weak |
| Microservices from day one | Independent scaling per domain | More code, more network hops, more operational cost | Poor |

## Recommended Architecture

Start with a `single stateless core service` and keep the deployment path simple.

### V1 Components

- `Listing service`: create, update, archive, and fetch listings
- `Search service`: indexed search by product text, location, price, and condition
- `Negotiation service`: offer/counter-offer messages and status transitions
- `Contact reveal service`: controlled phone-number release
- `Audit log`: track important actions for debugging and trust
- `MCP adapter`: agent-facing tool layer over the same service operations
- `Mobile app clients`: Android and iOS apps over the same backend contract
- `Mobile AI agent integration`: app-managed user agent using `openrouter/free`

### Minimal Deployment

| Layer | Recommendation | Reason |
| --- | --- | --- |
| Core API | Single stateless service | Simple deploy and horizontal scaling |
| MCP | Thin adapter layer | Easier desktop-agent integration without duplicating business logic |
| Mobile apps | Android and iOS clients | Direct user access without needing a website |
| Mobile agent provider | `openrouter/free` at app layer | Gives users a free first agent path |
| Database | One primary relational database | Strong consistency for deal and reveal states |
| Cache | In-memory cache or Redis only if needed | Keep repeated reads off the database |
| Auth | Token-based service auth | Clean boundary for agents and future users |

There is no image storage layer in V1. `picture_urls` are pass-through fields only.

## Why Relational Database First

| Choice | Pros | Cons |
| --- | --- | --- |
| Relational DB | Strong state integrity, easier transactional updates, good audit support | Slightly more schema work |
| Document DB only | Flexible JSON storage, quick early iteration | Harder to enforce negotiation and reveal correctness |

The listing body can still remain JSON-like, but the system state should live in a relational model.

## Throughput Reality

`50k+ RPS` is realistic only if the majority of requests are:

- lightweight reads
- token validation
- listing fetch/search with cache help
- simple negotiation transitions

`50k+ RPS` is not a safe assumption if every request performs:

- synchronous disk-heavy writes
- expensive SQL scans
- cross-service calls
- blocking third-party API work

## Recommended V1 APIs

- `POST /v1/listings`
- `GET /v1/listings`
- `GET /v1/listings/{id}`
- `GET /v1/listings/search`
- `POST /v1/negotiations`
- `POST /v1/negotiations/{id}/offers`
- `POST /v1/negotiations/{id}/request-contact-reveal`
- `POST /v1/contact-reveals/{id}/approve`

## Client Surface Rule

All three product surfaces must converge on the same business rules:

- server HTTP API
- MCP tools for desktop agents
- Android and iOS app flows

Mobile app agent features should not bypass marketplace validation or negotiation rules.

## Recommended V1 MCP Tools

- `create_listing`
- `search_listings`
- `get_listing`
- `open_negotiation`
- `submit_offer`
- `request_contact_reveal`
- `get_negotiation_status`

## Reliability Notes

- Use explicit statuses such as `draft`, `active`, `reserved`, `sold`, `archived`
- Make offer submission idempotent with client request IDs
- Never expose phone numbers in listing reads
- Log every state change with timestamp and actor ID
- Avoid synchronous work that is not required for the request response
- MCP tools should call the same internal service methods as the HTTP API
- Search queries must hit indexed dimensions, not open-ended scans
