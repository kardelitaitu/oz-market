# Decision Log

## Goal

Track accepted, rejected, and deferred architecture decisions so the codebase does not silently drift.

## Status Meanings

| Status | Meaning |
| --- | --- |
| `accepted` | chosen direction for current implementation |
| `rejected` | considered and intentionally not chosen |
| `deferred` | unresolved and intentionally delayed |

## Accepted Decisions

### D-001: Single Repo

- `status`: accepted
- `decision`: keep one repo for server, MCP, Android, and iOS
- `reason`: simpler coordination, easier contract sharing, cleaner early-stage delivery

### D-002: Server-First Product Shape

- `status`: accepted
- `decision`: product is `server + MCP + Android + iOS`, not website-first
- `reason`: matches agent-first workflow and avoids premature UI cost

### D-003: Rust Backend

- `status`: accepted
- `decision`: use `Rust` for backend implementation
- `reason`: strongest fit for throughput target and compact service design

### D-004: PostgreSQL As Source Of Truth

- `status`: accepted
- `decision`: use `PostgreSQL` with typed columns plus JSONB
- `reason`: strong consistency, good indexing, flexible contract support

### D-005: MCP As Thin Adapter

- `status`: accepted
- `decision`: MCP must be a thin adapter over core backend logic
- `reason`: avoids duplicated business rules and permission drift

### D-006: Frozen AI-Facing Listing Contract

- `status`: accepted
- `decision`: freeze the V1 listing payload at `schema_version = 1.0`
- `reason`: reduces agent and client drift across HTTP, MCP, Android, and iOS

### D-007: Reservation Lease Model

- `status`: accepted
- `decision`: use short-lived reservation leases for near-close flow
- `reason`: reduces double-sell risk and makes reveal/finalization deterministic

### D-008: Hardware ID Not Primary Identity

- `status`: accepted
- `decision`: use `hardware_id` only as optional abuse signal
- `reason`: too weak and unreliable for cloud/container agent environments

### D-009: PostgreSQL-First Search

- `status`: accepted
- `decision`: start with PostgreSQL indexing before external search engine
- `reason`: keeps codebase compact and avoids premature operational complexity

### D-010: openrouter/free At App Layer

- `status`: accepted
- `decision`: keep `openrouter/free` integration at app layer, not core backend
- `reason`: avoids provider logic leaking into marketplace rules

## Rejected Decisions

### D-101: Website-First Marketplace

- `status`: rejected
- `decision`: do not start with traditional website-first marketplace
- `reason`: slows backend and agent-first delivery

### D-102: Separate Business Logic For MCP

- `status`: rejected
- `decision`: do not implement MCP-specific business rules
- `reason`: high divergence risk and higher maintenance

### D-103: Hardware ID As Main Trust Anchor

- `status`: rejected
- `decision`: do not require hardware identity as primary anti-spam control
- `reason`: spoofability, portability problems, weak cloud support

### D-104: External Search Engine From Day One

- `status`: rejected
- `decision`: do not start with Elasticsearch/OpenSearch
- `reason`: unnecessary operational overhead before PostgreSQL limits are proven

### D-105: Microservices From Day One

- `status`: rejected
- `decision`: do not split server into microservices initially
- `reason`: higher operational and code complexity without proven need

## Deferred Decisions

### D-201: MCP Deployment Shape

- `status`: accepted
- `decision`: run MCP as a separate stdio sidecar binary
- `reason`: keeps desktop-agent transport isolated while sharing the same backend service logic; simpler operational boundary than embedding MCP in the HTTP runtime

### D-202: Money Precision Policy

- `status`: deferred
- `decision`: whether `price.amount` remains simple numeric or moves to stricter minor-unit representation
- `reason`: depends on real payment and accounting requirements
- `owner`: `dev (interim)`
- `target_stage`: before database migrations are finalized

### D-203: Location Normalization Depth

- `status`: deferred
- `decision`: whether `location.city` remains free text or becomes normalized
- `reason`: depends on search quality and data-cleaning needs
- `owner`: `dev (interim)`
- `target_stage`: before search ranking and indexing implementation hardens

### D-204: Provider Fallback Beyond openrouter/free

- `status`: deferred
- `decision`: whether mobile apps support more than `openrouter/free` in V1
- `reason`: depends on product and cost pressure
- `owner`: `dev (interim)`
- `target_stage`: before mobile agent settings are finalized

### D-205: Support Reviewer Access Depth

- `status`: deferred
- `decision`: how much negotiation detail support reviewers may inspect
- `reason`: depends on privacy and operations policy
- `owner`: `dev (interim)`
- `target_stage`: before support tooling or admin surfaces are implemented

## Update Rule

Whenever a major architecture or product decision changes:

1. update this decision log
2. update the affected whitepaper docs
3. update specs if the contract changed

For deferred decisions:

1. the listed owner should make the call before the target stage
2. once decided, move the item to `accepted` or `rejected`
