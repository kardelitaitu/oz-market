# Roadmap

## Phase 1: Foundation

- Define API contracts
- Define MCP tool contracts
- Define mobile app contract and user-agent boundaries
- Create minimal relational schema
- Implement listing CRUD
- Implement indexed search by text, location, price, and condition
- Benchmark the read path early

## Phase 2: Negotiation

- Offer and counter-offer flow
- Negotiation state machine
- Reservation lease and versioning rules
- Audit event tracking
- Idempotency and retry safety
- Expose negotiation actions through MCP
- Expose negotiation actions to Android and iOS app flows

## Phase 3: Contact Reveal

- Store private seller contact separately
- Reveal request and approval flow
- Expiring reveal records

## Phase 4: Hardening

- Auth and rate limits
- Observability and error tracking
- Duplicate listing and abuse controls
- Anti-spam controls for fake listings and agent abuse
- Load test toward the throughput target
- MCP tool-level validation and usage analytics
- Harden mobile app agent integration and provider fallback handling
- Revisit external search only if PostgreSQL indexing becomes the bottleneck

## Best Next Moves

1. Freeze the `listing JSON contract` so agents integrate against a stable payload.
2. Define the `negotiation state machine` before writing backend code.
3. Lock the base stack: `Rust + PostgreSQL + typed columns + JSONB`.
4. Write an `OpenAPI spec` before implementation so agent integration stays predictable.
5. Write the first `MCP tool manifest` so weaker agents have a safer integration path.
6. Define the benchmark scenario for `50k+ RPS` before promising that target.
7. Define the first `search query contract` around indexed dimensions only.
8. Define reservation and anti-spam rules before implementing offer acceptance.
9. Define the Android/iOS app contract and how user-created `openrouter/free` agents are configured safely.
10. Freeze required and optional listing fields plus `category` and `condition` enums.
11. Publish one canonical API contract for HTTP, MCP, and mobile clients.
12. Define seller identity, agent credential issuance, and permission rules.
13. Publish a first OpenAPI-oriented endpoint outline from the frozen contract.
14. Publish a strict role-permission matrix.
15. Publish explicit state machines for listing, negotiation, reservation, and reveal.
16. Publish non-functional requirements and error-budget targets.
17. Publish explicit risks and assumptions before implementation starts.
18. Publish a decision log for accepted, rejected, and deferred architecture choices.
19. Publish an implementation checklist that turns the whitepaper into execution order.
20. Publish a test strategy before implementation branches too far.
21. Define machine validation and compatibility rules for the API spec.
22. Define explicit auth scopes and token-claim shape for HTTP, MCP, and mobile.
23. Define admin and support operational surfaces before internal tooling grows ad hoc.
24. Define event delivery strategy for polling, webhooks, and push flows later if needed.
25. Define audit-event and outbox boundaries before asynchronous delivery grows.

## Alternatives To Consider

| Alternative | Pros | Cons |
| --- | --- | --- |
| Start with backend API only | Fastest path, best for agent-first system | No human-facing fallback at first |
| Keep a minimal seller dashboard early | Easier manual operations and testing | Slightly slower performance-focused progress |
| Start with chat-platform bot interface | Very easy early interaction model | Harder long-term system control and throughput predictability |

## Open Questions

- Is `location.country_code`, `location.country_name`, and `location.city` free text or partially normalized?
- Is `currency` fixed to ISO 4217 values in V1?
- Is contact reveal always a phone number, or also chat handles?
- Can one seller have multiple AI agents?
- Does the marketplace eventually support escrow or payment links?
- What exact workload defines the `50k+ RPS` target: reads, writes, or mixed traffic?
- Will the MCP server run inside the main binary or as a thin sidecar over the API?
- Is device identity optional evidence only, or a hard requirement for seller onboarding?
- Is `openrouter/free` the default free-agent path only, or the only app-agent provider in V1?
