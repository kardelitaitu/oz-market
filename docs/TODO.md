# TODO

last audited 05-06-26 by docs-auditor

## Status: MAJOR DRIFT FOUND

The codebase is functionally correct and all 6 CI gates pass, but three independent audits found real drift between code, OpenAPI spec, and done-spec plans. **None of the drift is a runtime bug** — the OpenAPI spec is treated as documentation, not as the source of truth, and the legacy raw-TCP runtime is not the production entry point. But the spec/code mismatch is a liability: a future client written against the spec will break on the legacy path, and the spec is silently decorative in some places (e.g. `CreateReviewRequest` is not bound by the handler).

| Audit | Scope | Drift count |
|---|---|---|
| OpenAPI vs HTTP handlers | `openapi.yaml` ↔ `actix_handlers.rs` + `runtime.rs` | 7 major, 7 minor |
| Env vars vs code + docs | `backend/server` env reads ↔ `.env.example` + README + deploy.md | 16 items (2 unused, 9 undocumented, 5 mismatches) |
| Done specs vs code | specs 0001-0018 ↔ backend | 1 major (0013), 5 minor |

## MAJOR — fix first

### M1. Pick one HTTP runtime

The repo has two parallel routers:

| Runtime | File | Production? |
|---|---|---|
| Actix | `backend/server/src/http/actix_handlers.rs` + `actix_runtime.rs` | Yes (the `run` exported from `mod.rs` is the Actix one in `actix_runtime.rs`; the legacy is exposed only via `pub use runtime::run`) |
| Legacy raw-TCP | `backend/server/src/http/runtime.rs` | **No, but reachable**: `mod.rs` re-exports `pub use runtime::run` so it can still be invoked |

The legacy runtime drifts from the spec on at least three admin paths:

- `POST /internal/v1/listings/{id}/release-reservation` (spec: `POST /internal/v1/reservations/{lease_id}/release`, no body)
- `POST /internal/v1/sellers/{id}/trust-level` (spec: `PUT`, raw string body)
- `POST /internal/v1/sellers/{id}/quota-override` (spec: `PUT`, raw int/null body)

**Fix:** delete `pub use runtime::run` from `mod.rs` and remove `backend/server/src/http/runtime.rs` after confirming no binary or test imports it. If it must stay, port the divergent routes to the Actix paths.

### M2. Document the agent + SSE + internal-ops routes in OpenAPI

Live routes that are missing from `openapi.yaml`:

- `POST /v1/agent/query` — `actix_handlers.rs:1389`
- `GET /v1/health/agents` — `actix_handlers.rs:1391`
- `GET /v1/health/agents/{agent_id}` — `actix_handlers.rs:1393`
- `POST /v1/health/agents/{agent_id}/reset` — `actix_handlers.rs:1397`
- `GET /v1/events/negotiations/{negotiation_id}` — `actix_handlers.rs:1462` (SSE; `NegotiationEvent` payload format not documented)
- `GET /internal/v1/rate-limits` — `actix_handlers.rs:1494`
- Deprecated redirects: `/v1/product/{id}`, `/v1/product/search`, `/v1/service/{id}`, `/v1/service/search`, `/v1/property/{id}`, `/v1/property/search` — all emit `Deprecation: true` + `Sunset: 2026-06-01`

**Fix:** add a new `agents` + `sse` + `internal-ops` tag block in `openapi.yaml`; mark the six deprecated redirects under a `deprecated: true` flag.

### M3. Add the four ledger metrics (spec 0013 §4)

Spec 0013 plan §4 required these Prometheus counters; none are emitted:

- `ledger_cache_hit_total`
- `ledger_cache_miss_total`
- `ledger_batch_lag_milliseconds`
- `ledger_batch_size`

**Fix:** add counters to `backend/server/src/observability/mod.rs` and emit them in `services/ledger_cache.rs` (hit/miss) + `services/async_committer.rs` (lag/size).

### M4. Document the spec 0017 health-API endpoints in OpenAPI

Spec 0017 plan §3 explicitly required `/v1/health/agents`, `/v1/health/agents/{agent_id}`, and `/v1/health/agents/{agent_id}/reset` to be in OpenAPI. They are absent.

**Fix:** add the three paths under an `agents` tag. Also update `docs/specs/_done/0017-agent-circuit-breaker-health-api/parity-report.md` (still shows PENDING despite being in `_done/`) or delete it.

### M5. Reconcile 200 vs 204 on six admin endpoints

Spec says `200`; code returns `204 No Content` (semantically correct since no body):

- `POST /internal/v1/reservations/{lease_id}/release` — `actix_handlers.rs:932`
- `PUT /internal/v1/sellers/{seller_id}/trust-level` — `actix_handlers.rs:959`
- `PUT /internal/v1/sellers/{seller_id}/quota-override` — `actix_handlers.rs:986`
- `POST /internal/v1/sellers/{seller_id}/recalculate-rating` — `actix_handlers.rs:1020`
- `POST /internal/v1/reviews/{review_id}/approve` — `actix_handlers.rs:1224`
- `POST /internal/v1/reviews/{review_id}/reject` — `actix_handlers.rs:1261`

**Fix:** update `openapi.yaml` to `204` for all six. `archive_listing` is already `204` and is the reference.

## MINOR — fix in next batch

### m1. Add `200` response for idempotency replay on `POST /v1/listings` and `POST /v1/negotiations`

Spec only documents `201`. Code (`actix_handlers.rs:524-533, 619-633`) returns `200` on `idempotency_key` replay with the same body shape. **Fix:** add `200: CreateListingResponse` and `200: OpenNegotiationResponse` entries.

### m2. Add `owner_id` to `GET /v1/listings/search` query parameters

`SearchRequest.owner_id` is supported in `api-contract/src/listing.rs:300-302` and read in `runtime.rs:1068`, but absent from the spec query parameter list. **Fix:** add `owner_id: string` to the parameter schema.

### m3. Document the `X-RateLimit-*` response headers

`actix_handlers.rs:734-737, 782-786, 827-833` emit `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset` on the three negotiation endpoints. **Fix:** add a shared header component in OpenAPI and reference it from each operation.

### m4. Document the SSE stream payload

`GET /v1/events/negotiations/{negotiation_id}` (`actix_handlers.rs:84-167`) emits `event: negotiation_updated` frames with a `NegotiationEvent { negotiation_id, event_type, response }` body. None of this is in the spec. **Fix:** add an SSE schema entry to the spec.

### m5. Type-bind `POST /v1/listings/{id}/reviews` to `CreateReviewRequest`

`actix_handlers.rs:1029-1111` currently uses `web::Json<serde_json::Value>` and manually reads `rating`/`title`/`body`, so the spec schema is not enforced — a rename in the spec won't break the build. **Fix:** switch the body type to `web::Json<CreateReviewRequest>` and use the typed fields.

### m6. Remove or move the duplicate archive route

`POST /v1/listings/{listing_id}/archive` is registered at `actix_handlers.rs:1431` in addition to the spec's `POST /internal/v1/listings/{listing_id}/archive`. Both require admin claims, so functionally safe. **Fix:** keep only the `/internal/v1/...` route (the spec-correct one).

### m7. Spec 0010 plan: document the string-keyed, multi-account design

Plan §1 said `agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE`. Migration `0014_add_credit_ledger.sql:3-8` uses `agent_id TEXT PRIMARY KEY` with no FK, and `CreditLedgerError::AgentNotFound(String)` (`domain/ledger.rs:54`) uses `String` not `Uuid`. The implementation is intentional (multi-tenant / guest agents), but the plan was never updated. **Fix:** append a `## Drift Notes` section to `0010/plan.md` describing the string-keyed, multi-account design and why.

### m8. Spec 0011 plan: cache stores full struct, not just `Decimal`

Plan §1 said the cache value is `Decimal` (current balance). Actual `LedgerCache` stores `CachedEntry { account: CreditAccount, cached_at: Instant }`. Functionally richer, but doc is stale. **Fix:** one-line edit to `0011/plan.md` §1.

### m9. Spec 0012 plan: stale path and file references

Plan refers to `backend/server/src/http/handlers.rs::admin_adjust_credits`; the function is now `adjust_credits` in `actix_handlers.rs:1539`. Plan also said `POST /v1/admin/sellers/{id}/credits`; current is `POST /internal/v1/sellers/{seller_id}/credits`. **Fix:** update `0012/plan.md` §3 to current path and file.

## ENV VAR DRIFT

### e1. `.env.example:4` says "All settings are optional"

Wrong for `DATABASE_URL` in the production server runtimes — `actix_runtime.rs:352` returns an error and refuses to start if unset. `docs/deploy.md:62` correctly marks it as required. **Fix:** rewrite the preamble to acknowledge the production-required flag.

### e2. `POPULATE_DB_SELLER_COUNT` is a dead name

`.env.example:67` and `.env:64` reference it, but the code reads `POPULATE_NUM_SELLERS` (`populate_db.rs:19`). Uncommenting `.env.example`'s line would have no effect. **Fix:** rename to `POPULATE_NUM_SELLERS` in both files.

### e3. `MARKETPLACE_PORT` is a no-op

`docker-compose.yml:31` maps it, but the server reads `MARKETPLACE_BIND` (`actix_runtime.rs:58`). Operators who set it expecting it to change the bind address will be confused. **Fix:** drop the `MARKETPLACE_PORT` indirection or add a comment pointing to `MARKETPLACE_BIND`.

### e4. `DATABASE_MAX_CONNECTIONS` default mismatch (highest-impact env drift)

Code default is `200` (`actix_runtime.rs:361`); `.env.example:14` says `200`; `backend/server/README.md:144` says **`100`**. An operator following the README halves the pool. **Fix:** update the README to `200`.

### e5. `TOKIO_WORKER_THREADS` default description incomplete

Code: `(cpus-1).max(1).min(8)` — `actix_runtime.rs:42-45`. `backend/server/README.md:145` describes it as `num_cpus - 1` without the cap. **Fix:** mention the cap of 8 in the README.

### e6. `MARKETPLACE_DISABLE_CACHE` semantics under-documented

Code accepts `"1"` or `"true"` (case-insensitive) — `actix_runtime.rs:87-89`. `docs/deploy.md:65` only mentions `"1"`. **Fix:** add `or true` to the deploy doc.

### e7. Nine env vars in code but absent from `.env.example` / README / deploy.md

| Var | Default | Consumer |
|---|---|---|
| `SHUTDOWN_TIMEOUT_SECS` | `30` | `actix_runtime.rs:171` |
| `LEDGER_WAL_PATH` | `./data/ledger.wal` | `wal.rs:78` |
| `LEDGER_CACHE_TTL_SECS` | `300` | `ledger_cache.rs:36` (security-relevant — only mentioned in `0012/.../decisions.md`) |
| `HTTP_BENCH_CONCURRENCY` (singular) | alias of `HTTP_BENCH_CONCURRENCIES` | `bench_concurrent.rs:88` |
| `POPULATE_LISTINGS_PER_SELLER` | `100` | `populate_db.rs:23` |
| `POPULATE_REVIEWS_PER_LISTING` | `1` | `populate_db.rs:27` |
| `PHASE5_BENCH_OPS` | `10_000` | `phase5_bench.rs:836` |
| `MARKETPLACE_BENCH_CLAIMS_JSON` | unset | `bench_concurrent.rs:169` |
| `HTTP_BENCH_CLAIMS_MODE` | `"rotating"` | `bench_concurrent.rs:126` |

**Fix:** add all nine to `.env.example` (with comments) and to a single `docs/server/environment.md` consolidation (the canonical env-var table is currently split across three docs that disagree in 5 places).

### e8. `docs/deploy.md:236` overstates `MARKETPLACE_API_KEY` scope

Says it "maps to full-access demo Claims for both HTTP and MCP". True for HTTP, but MCP reads `MARKETPLACE_MCP_CLAIMS_JSON` / `MARKETPLACE_MCP_ALLOW_DEV_CLAIMS` instead. **Fix:** drop "and MCP" from the description.

## AMBIGUOUS / NOTED

- A checked-in `.env` file exists at the repo root. It contains no real secrets (only a localhost `DATABASE_URL`), but its presence is a configuration-management smell. Consider `.gitignore`.
- `LOG_FORMAT=json` activates JSON logging; other values silently fall back to `text` rather than rejecting. The doc says values are `"text"` or `"json"` — the `"text"` value is effectively a no-op default, not a real setting.

## FILES TOUCHED (quick reference)

- `docs/specs/openapi.yaml`
- `docs/specs/_done/0010-credit-ledger-schema-domain/plan.md`
- `docs/specs/_done/0011-dual-layer-ledger-cache/plan.md`
- `docs/specs/_done/0012-ledger-cache-invalidation/plan.md`
- `docs/specs/_done/0013-ledger-async-batch-wal/{plan.md,spec.yaml}`
- `docs/specs/_done/0017-agent-circuit-breaker-health-api/{plan.md,parity-report.md,spec.yaml}`
- `.env.example`, `.env`
- `docker-compose.yml`
- `docs/deploy.md`
- `backend/server/README.md`
- `backend/server/src/http/actix_handlers.rs` (m5, m6)
- `backend/server/src/http/runtime.rs` (M1 — delete or port)
- `backend/server/src/http/mod.rs` (M1 — drop `pub use runtime::run`)
- `backend/server/src/observability/mod.rs` (M3 — add counters)
- `backend/server/src/services/ledger_cache.rs` (M3 — emit hit/miss)
- `backend/server/src/services/async_committer.rs` (M3 — emit lag/size)

## PROPOSED NEXT MOVES

1. **M1** — remove the legacy raw-TCP runtime (`pub use runtime::run` from `mod.rs` + delete `runtime.rs`). Single biggest risk reduction. After this, M2-m6 all reference a single source of truth.
2. **M3** — add the four ledger metrics. Small, contained, high observability value.
3. **M5** — flip the six 200→204 in the spec. One-line edits, no code change.
4. **M2** — add the missing OpenAPI paths (agents, SSE, internal-ops, deprecated). Larger edit but pure documentation.
5. **e1-e8** — env-var cleanup batch. Update `.env.example`, `.env`, README, `deploy.md`. Pure documentation.

Production deployment items (from previous TODO) that are NOT part of this audit:

- Deploy to a Linux VPS
- Configure domain + reverse proxy
- PostgreSQL backups
- Log aggregation (Loki / Datadog)
- Mobile app stores (Android / iOS)
- MCP HTTP/SSE transport (currently stdio-only)
- End-to-end benchmark CI gate
