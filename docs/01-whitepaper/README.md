# Project Whitepaper

This folder contains the planning docs for a minimal `AI-agent marketplace bridge` for new and used products.

## End Product

The end product is:

1. a `server`
2. an `MCP server` for desktop agents
3. `Android` and `iOS` apps with a user-created free AI agent powered by `openrouter/free`

## Core Idea

- Sellers publish product listings with a very small payload.
- Buyer AI agents search, compare, and negotiate.
- The service acts as a `bridge` between two agents, not a media platform.
- Seller AI agents do not reveal direct contact details early.
- When a deal is close, the seller side reveals a phone number to the buyer side.

## Recommended Starting Shape

Start with a `server-first architecture` and treat any traditional web marketplace UI as out of scope for V1.

| Option | Pros | Cons |
| --- | --- | --- |
| Server + MCP + mobile apps | Matches the actual product shape, covers both agents and end users | More integration surfaces than server-only |
| Website-first marketplace | Easier for manual human browsing on day one | Adds frontend cost and hurts throughput focus |

## Document Map

- [01-overview.md](01-overview.md): product scope, goals, and constraints
- [02-architecture-options.md](02-architecture-options.md): deployment shape and recommended architecture
- [03-data-model.md](03-data-model.md): minimal data model and storage boundaries
- [04-agent-transaction-flow.md](04-agent-transaction-flow.md): listing, negotiation, and contact reveal flow
- [05-roadmap.md](05-roadmap.md): phased build plan and next decisions
- [06-performance-and-stack.md](06-performance-and-stack.md): throughput target, stack choice, and hot-path rules
- [07-mcp-server.md](07-mcp-server.md): MCP integration strategy for buyer and seller agents
- [08-search-indexing.md](08-search-indexing.md): fast product search via smart multi-dimensional indexing
- [09-concurrency-and-reservations.md](09-concurrency-and-reservations.md): reservation leases, versioning, anti-spam controls, and double-sell prevention
- [10-api-contract.md](10-api-contract.md): canonical HTTP, MCP, and mobile contract for listing create/get/search
- [11-identity-authz.md](11-identity-authz.md): seller identity, agent credentials, permissions, and anti-abuse trust model
- [12-openapi-outline.md](12-openapi-outline.md): implementation-oriented API outline derived from the frozen contract
- [13-role-permission-matrix.md](13-role-permission-matrix.md): strict role-to-endpoint and role-to-action permission matrix
- [14-state-machines.md](14-state-machines.md): listing, negotiation, reservation, and reveal transition rules
- [15-non-functional-requirements.md](15-non-functional-requirements.md): latency, throughput, quotas, availability, and error budget targets
- [16-risks-and-assumptions.md](16-risks-and-assumptions.md): explicit unresolved assumptions, risks, and mitigation direction
- [17-decision-log.md](17-decision-log.md): accepted, rejected, and deferred architecture decisions
- [18-implementation-checklist.md](18-implementation-checklist.md): phased execution checklist from whitepaper to first implementation
- [19-test-strategy.md](19-test-strategy.md): validation strategy for contracts, authz, state transitions, concurrency, and performance
- [20-spec-validation-and-governance.md](20-spec-validation-and-governance.md): machine validation, compatibility rules, and spec change control
- [21-auth-scopes-and-claims.md](21-auth-scopes-and-claims.md): token scopes, claims, and role-to-scope mapping
- [22-admin-and-support-surfaces.md](22-admin-and-support-surfaces.md): internal admin and support endpoints plus operational boundaries
- [23-event-delivery.md](23-event-delivery.md): polling, webhooks, and event-stream strategy for MCP and mobile clients
- [24-audit-events-and-outbox.md](24-audit-events-and-outbox.md): audit logging, outbox delivery, and reliable event publication boundaries

## Recommended Reading Order

For a new contributor, use this sequence:

1. [01-overview.md](01-overview.md)
2. [02-architecture-options.md](02-architecture-options.md)
3. [10-api-contract.md](10-api-contract.md)
4. [11-identity-authz.md](11-identity-authz.md)
5. [13-role-permission-matrix.md](13-role-permission-matrix.md)
6. [14-state-machines.md](14-state-machines.md)
7. [09-concurrency-and-reservations.md](09-concurrency-and-reservations.md)
8. [08-search-indexing.md](08-search-indexing.md)
9. [15-non-functional-requirements.md](15-non-functional-requirements.md)
10. [16-risks-and-assumptions.md](16-risks-and-assumptions.md)
11. [17-decision-log.md](17-decision-log.md)
12. [18-implementation-checklist.md](18-implementation-checklist.md)
13. [19-test-strategy.md](19-test-strategy.md)
14. [20-spec-validation-and-governance.md](20-spec-validation-and-governance.md)
15. [21-auth-scopes-and-claims.md](21-auth-scopes-and-claims.md)
16. [22-admin-and-support-surfaces.md](22-admin-and-support-surfaces.md)
17. [23-event-delivery.md](23-event-delivery.md)
18. [24-audit-events-and-outbox.md](24-audit-events-and-outbox.md)
19. [05-roadmap.md](05-roadmap.md)

## Default Recommendation

Build a small bridge service with:

- `core server`
- `HTTP JSON API`
- `search` API
- `negotiation` API
- `contact reveal` gate
- `audit/event log`
- `MCP server` for desktop agents
- `Android` and `iOS` apps
- mobile user agent powered by `openrouter/free`
- `smart search indexing`

Rules:

- no image uploads
- no image proxying
- no server-rendered website in the core service
- only store listing metadata and negotiation state
- MCP must be a thin layer over the same core business logic
- mobile apps should call the same core backend contract
- user-created AI agents should stay configurable at the app layer, not fork backend logic

This is the smallest shape that still supports reliability, scale, and safe contact handoff.
