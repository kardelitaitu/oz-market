# Risks And Assumptions

## Goal

Make unresolved design assumptions explicit before implementation starts.

This reduces hidden disagreement and prevents the codebase from embedding accidental policy.

## Current Assumptions

### Product Assumptions

- the end product is `server + MCP + Android + iOS`
- traditional website marketplace UI is not required for V1
- buyer and seller agents should use the same backend rules
- MCP is delivered as a separate stdio sidecar binary, not embedded in the HTTP server

### Contract Assumptions

- the V1 AI-facing listing payload is frozen at `schema_version = 1.0`
- `category` and `condition` enums are stable enough for V1
- `price.amount` is sufficient for V1 money handling

### Search Assumptions

- PostgreSQL-first indexing is enough for V1
- deterministic search ordering matters more than advanced ranking
- search should stay inside indexed dimensions only in V1

### Auth Assumptions

- seller identity is the trust anchor
- agent credentials are required for non-human actions
- `hardware_id` is optional abuse evidence only

## Key Risks

### 1. Throughput Risk

The repo currently targets `50k+ RPS`, but that number is only realistic for a narrow request mix.

Risk:

- team optimizes for an unrealistic blended target

Mitigation:

- benchmark read, search, and write paths separately

### 2. Contract Drift Risk

Multiple surfaces exist:

- HTTP
- MCP
- Android
- iOS

Risk:

- each surface starts using a slightly different payload or state interpretation

Mitigation:

- enforce one canonical contract and shared service logic

### 3. Permission Drift Risk

Risk:

- MCP or mobile app flows become less strict than server flows

Mitigation:

- centralize authz and enforce one role-permission model

### 4. Concurrency Risk

Risk:

- double-sell or overlapping reveal approval under race conditions

Mitigation:

- reservation lease model
- optimistic versioning
- transactional state changes

### 5. Abuse Risk

Risk:

- fake listings, spam negotiations, replay storms

Mitigation:

- quotas
- rate limits
- duplicate listing fingerprints
- seller trust levels

### 6. Provider Dependency Risk

Risk:

- `openrouter/free` changes behavior, limits, or availability

Mitigation:

- keep provider integration at app layer
- design provider fallback path later

## Open Decisions

- whether `currency` is the only money normalization needed in V1
- whether `location.city` is normalized or free text
- whether mobile agent provider remains only `openrouter/free` in V1
- whether support reviewers can access negotiation detail beyond read-only views

## Assumption Review Rule

Before implementation of a new area, re-check:

- contract assumptions
- auth assumptions
- throughput assumptions
- provider dependency assumptions

## Best Next Moves

1. convert open decisions into explicit accepted or rejected decisions
2. attach one owner to each major risk
3. revisit this doc before backend scaffolding begins
