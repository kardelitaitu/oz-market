# Implementation Checklist

## Goal

Turn the whitepaper into an execution order that can be followed without inventing architecture during coding.

## Phase 0: Freeze Inputs

- [x] ~~confirm `schema_version = 1.0` listing payload stays unchanged~~ (actual: the frozen contract and spec examples already use `1.0`)
- [x] ~~confirm `category` and `condition` enums~~ (actual: the frozen contract and server code already use the V1 enums)
- [x] ~~confirm `price.amount` policy for V1~~ (actual: the frozen contract and schema already use a numeric price amount)
- [x] ~~confirm initial seller onboarding and trust levels~~ (actual: verified seller accounts start in `new`, move through `verified` and `trusted`, and can fall back to `restricted`)
- [x] ~~confirm deferred decision owners accept their ownership~~ (actual: the deferred decisions are already tracked with interim owners in the decision log)

## Phase 1: Contract And Auth Foundations

- [x] ~~finalize `docs/specs/openapi.yaml`~~ (actual: the frozen contract and validation gates are already wired)
- [x] ~~define spec lint and compatibility gates from `20-spec-validation-and-governance.md`~~ (actual: yamllint, Redocly, Spectral, and oasdiff are wired)
- [x] ~~align MCP tool shapes with the same contract~~ (actual: MCP tool catalog follows the frozen contract)
- [x] ~~finalize role-permission enforcement from `13-role-permission-matrix.md`~~ (actual: auth-core and server authz enforce the matrix)
- [x] ~~finalize token and agent credential model from `11-identity-authz.md`~~ (actual: token claims and credential model are wired)
- [x] ~~finalize auth scopes and token claims from `21-auth-scopes-and-claims.md`~~ (actual: auth scopes and claims are implemented)
- [x] ~~define machine-readable error codes as shared constants~~ (actual: shared API error codes are in place)

## Phase 2: Data And State Foundations

- [x] ~~finalize relational schema for `listings`~~ (actual: listings table is in the frozen migration)
- [x] ~~finalize relational schema for `negotiations`~~ (actual: negotiations table is in the frozen migration)
- [x] wire Postgres-backed `reservation_leases`
- [x] wire Postgres-backed `contact_reveals`
- [x] ~~wire Postgres-backed `audit_events`~~ (actual: server runtime uses the Postgres audit repository)
- [x] ~~wire Postgres-backed `outbox_events`~~ (actual: server runtime uses the Postgres outbox repository)
- [x] ~~add optimistic `version` fields to stateful entities~~ (actual: listings and negotiations already carry version columns)
- [ ] encode state transitions from `14-state-machines.md`

## Phase 3: Abuse And Quota Controls

- [ ] implement seller trust levels
- [ ] implement listing quotas by trust level
- [ ] implement per-token and per-IP rate limits
- [ ] implement duplicate listing fingerprinting
- [ ] implement idempotency enforcement on replay-sensitive writes
- [ ] implement reservation conflict handling

## Phase 4: Server Delivery

- [x] ~~scaffold Rust workspace under `backend/`~~ (actual: backend workspace is already present)
- [x] ~~implement shared contract types in `backend/crates/api-contract`~~ (actual: shared typed contract crate is already present)
- [x] ~~implement auth helpers in `backend/crates/auth-core`~~ (actual: shared auth helper crate is already present)
- [x] ~~keep shared marketplace logic inside the server service layer for V1~~ (actual: HTTP and MCP share the same app/services)
- [x] ~~implement HTTP transport in `backend/server`~~ (actual: HTTP runtime and routes are already wired)
- [ ] implement internal admin/support transport boundaries from `22-admin-and-support-surfaces.md`
- [x] ~~add database migrations~~ (actual: migration files are already wired)
- [ ] add tracing and metrics hooks
- [x] ~~add audit and outbox write paths for stateful mutations~~ (actual: stateful writes already emit audit and outbox events)

## Phase 5: MCP Delivery

- [x] ~~implement MCP transport in `backend/mcp`~~ (actual: MCP crate already exposes the shared facade)
- [x] ~~map all MCP tools to shared service functions~~ (actual: MCP handlers delegate into shared app logic)
- [x] ~~verify MCP cannot bypass authz or abuse controls~~ (actual: MCP paths reuse the same authz and guard checks)
- [ ] add MCP conflict and retry examples
- [ ] define MCP event consumption model from `23-event-delivery.md`

## Phase 6: Mobile Delivery

- [x] ~~Android contract scaffold and first-flow shell~~
- [x] ~~Android setup scaffold and first-flow shell~~
- [x] ~~Android UI shell for first flows~~
- [x] ~~iOS contract scaffold and first-flow shell~~
- [x] ~~iOS setup scaffold and first-flow shell~~
- [x] ~~iOS UI shell for first flows~~
- [x] ~~mobile seller identity scaffold mapping to `seller_account_id`~~
- [x] ~~mobile agent credential and short-lived session scaffold~~
- [x] ~~mobile `openrouter/free` setup scaffold~~
- [x] ~~ensure mobile clients use canonical backend payloads only~~
- [x] ~~define mobile polling-first event integration from `23-event-delivery.md`~~

## Phase 7: Search And Performance

- [x] ~~implement first PostgreSQL indexes~~ (actual: indexes already exist in the frozen migration)
- [x] ~~define benchmark profiles from `15-non-functional-requirements.md`~~ (actual: the benchmark profiles are already defined in `15-non-functional-requirements.md`)
- [x] ~~run listing-read benchmark~~ (actual: executed via `phase5_bench`, in-memory fallback because `DATABASE_URL` was unset)
- [x] ~~run search-heavy benchmark~~ (actual: executed via `phase5_bench`, in-memory fallback because `DATABASE_URL` was unset)
- [x] ~~run negotiation-burst benchmark~~ (actual: executed via `phase5_bench`, in-memory fallback because `DATABASE_URL` was unset)
- [ ] rerun `phase5_bench` against Postgres-backed storage with `backend/server/scripts/run-phase5-bench.ps1`
- [ ] adjust quotas from measured behavior for new seller daily/hourly limits, per-token create writes, and per-IP search pressure
- [ ] adjust indexes from measured behavior for `idx_listings_search_text`, `idx_listings_category_status`, and `idx_listings_location`

## Phase 8: Final Readiness Checks

- [x] ~~verify all deferred decisions are resolved or consciously postponed~~ (actual: D-201 through D-205 are recorded as deferred with owners and target stages in the decision log)
- [x] ~~verify docs/specs/code use the same contract names~~ (actual: the frozen contract, specs, whitepaper, and server code now use the same listing, negotiation, reservation, reveal, and idempotency names)
- [x] ~~verify all write paths are idempotent where required~~ (actual: create, open negotiation, and contact reveal replay tests pass, and repeat state transitions fail cleanly)
- [x] ~~verify reservation and reveal invariants hold under race conditions~~ (actual: concurrent open-negotiation and contact-approval tests show one winner and one conflict)
- [x] ~~verify observability signals exist for quotas, conflicts, and errors~~ (actual: the server observability snapshot now tracks request, internal write, conflict, quota rejection, and error counts)

## Exit Criteria For First Implementation

- [ ] one canonical contract across HTTP, MCP, Android, and iOS
- [ ] one shared backend business-rule layer
- [ ] explicit authz and abuse controls
- [ ] deterministic state transitions
- [ ] measured baseline performance, not only planned performance
