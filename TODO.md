# TODO

This session tracks only the remaining open work.
Completed foundation items are archived in [archive/TODO-2026-05-05.md](archive/TODO-2026-05-05.md).

Archive convention:
- when a session is closed, move completed work into `archive/` with a dated filename
- keep this root TODO focused on only the remaining open work
- until actual people are assigned, use the decision-log interim owners

## Focus

1. Resolve the last product and ownership decisions.
2. Finish the remaining hardening and client-delivery gaps.
3. Keep all remaining work contract-first and transport-safe.

## Open Work

### Phase 0: Product Decisions

- [x] ~~define seller onboarding policy for V1~~ (actual: V1 uses verified seller accounts, low-trust startup quotas, short-lived agent credentials, and trust_review_required for risky actions)
- [x] ~~define trust-level progression for new sellers~~ (actual: V1 uses new -> verified -> trusted -> restricted)

### Phase 0b: Governance

- [x] ~~assign `dev` as code owner for MCP deployment shape (`D-201`)~~ (actual: the decision log already records `dev (interim)` for D-201)
- [x] ~~assign `dev` as code owner for money precision policy (`D-202`)~~ (actual: the decision log already records `dev (interim)` for D-202)
- [x] ~~assign `dev` as code owner for location normalization depth (`D-203`)~~ (actual: the decision log already records `dev (interim)` for D-203)
- [x] ~~assign `dev` as product owner for provider fallback beyond `openrouter/free` (`D-204`)~~ (actual: the decision log already records `dev (interim)` for D-204)
- [x] ~~assign `dev` as admin owner for support reviewer access depth (`D-205`)~~ (actual: the decision log already records `dev (interim)` for D-205)
- [x] ~~confirm deferred decision owners accept their ownership~~ (actual: the deferred decisions are already tracked with interim owners in the decision log)

### Phase 0c: Product Ownership

- [x] ~~assign `dev` as product owner for seller onboarding policy~~ (actual: the onboarding policy is now defined and the decision log still uses `dev (interim)` ownership)
- [x] ~~assign `dev` as product owner for trust-level progression for new sellers~~ (actual: the trust-level progression is now defined and the decision log still uses `dev (interim)` ownership)

### Phase 1: State And Transition Gaps

- [x] ~~encode listing state transitions from `14-state-machines.md`~~
- [x] ~~encode negotiation state transitions from `14-state-machines.md`~~
- [x] ~~encode reservation state transitions from `14-state-machines.md`~~
- [x] ~~encode contact reveal state transitions from `14-state-machines.md`~~

### Phase 2: Server Hardening

- [x] ~~define internal `/internal/v1` route namespace from `22-admin-and-support-surfaces.md`~~
- [x] ~~define internal access scopes and audit rules from `22-admin-and-support-surfaces.md`~~
- [x] ~~add tracing and metrics hooks~~

### Phase 3: MCP Finishing

- [x] ~~add MCP conflict and retry examples~~
- [x] ~~define MCP event consumption model from `23-event-delivery.md`~~

### Phase 4: Mobile Delivery

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

### Phase 5: Search And Performance

- [x] ~~define benchmark profiles from `15-non-functional-requirements.md`~~ (actual: the benchmark profiles are already defined in `15-non-functional-requirements.md`)
- [x] ~~run listing-read benchmark~~ (actual: executed via `phase5_bench`, in-memory fallback because `DATABASE_URL` was unset)
- [x] ~~run search-heavy benchmark~~ (actual: executed via `phase5_bench`, in-memory fallback because `DATABASE_URL` was unset)
- [x] ~~run negotiation-burst benchmark~~ (actual: executed via `phase5_bench`, in-memory fallback because `DATABASE_URL` was unset)
- [x] ~~rerun `phase5_bench` against Postgres-backed storage with `backend/server/scripts/run-phase5-bench.ps1`~~ (DONE: 321/77/85 ops/sec for listing-read/search-heavy/negotiation-burst)
- [x] adjust quotas from measured behavior for new seller daily/hourly limits, per-token create writes, and per-IP search pressure
- [x] adjust indexes from measured behavior for `idx_listings_search_text`, `idx_listings_category_status`, and `idx_listings_location`

### Phase 6: Final Readiness Checks

- [x] ~~verify all deferred decisions are resolved or consciously postponed~~ (actual: D-201 through D-205 are recorded as deferred with owners and target stages in the decision log)
- [x] ~~verify docs/specs/code use the same contract names~~ (actual: the frozen contract, specs, whitepaper, and server code now use the same listing, negotiation, reservation, reveal, and idempotency names)
- [x] ~~verify all write paths are idempotent where required~~ (actual: create, open negotiation, and contact reveal replay tests pass, and repeat state transitions fail cleanly)
- [x] ~~verify reservation and reveal invariants hold under race conditions~~ (actual: concurrent open-negotiation and contact-approval tests show one winner and one conflict)
- [x] ~~verify observability signals exist for quotas, conflicts, and errors~~ (actual: the server observability snapshot now tracks request, internal write, conflict, quota rejection, and error counts)

### Phase 7: Production Hardening

- [ ] implement database connection pooling for production deployment
- [ ] add rate limiting middleware for API endpoints
- [ ] implement comprehensive error handling and logging
- [ ] add database migration rollback capabilities
- [ ] implement health checks for external dependencies (AI providers, database)
- [ ] add request/response compression for performance
- [ ] implement graceful shutdown handling
- [ ] add database query optimization and connection limits
- [ ] implement circuit breaker pattern for external API calls
- [ ] add production-ready configuration management (secrets, environment variables)

## Working Rule

- update this TODO when build order changes
- prefer contract-first implementation
- do not start transport code before validation policy is executable
