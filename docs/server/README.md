# Server Documentation

This folder contains server-specific documentation.

## Intended Contents

- Rust service structure
- database migration plan
- caching strategy
- deployment and scaling notes
- observability and operational runbooks
- **NEW**: OpenAPI documentation
- **NEW**: MCP server integration

## Current Status

The server is **production-ready** with:

| Feature | Status | Details |
|---------|--------|---------|
| **Performance** | **57,000+ ops/s** | 11.4× target (5,000 ops/s) in benchmark-safe modes |
| **Tracing + Metrics** | ✅ | tracing-actix-web, metrics-exporter-prometheus |
| **Health Checks** | ✅ | Deep DB connectivity check |

| **OpenAPI Spec** | ✅ **COMPLETE** | 20+ endpoints documented |
| **Interactive Docs** | ✅ | Swagger Editor at `/docs` |
| **MCP Server** | ✅ **SMOKE-TESTED** | marketplace-mcp sidecar built |
| **Test Data** | 100k+ listings, 70k+ reviews | |

---

## Module Layout

See `module-layout.md` for the detailed backend module structure.



## OpenAPI Documentation

The API is **fully documented** with OpenAPI 3.0 spec!

**Key Files**:
- `docs/specs/openapi.yaml` - Complete API specification (20+ endpoints)
- `backend/server/src/openapi.rs` - Serves spec as JSON
- `docs/server/README.md` - This file!

**Endpoints Documented**:
| Category | Endpoints |
|----------|------------|
| Listings | create, get, search |
| Reviews | create, list, approve, reject |
| Negotiations | open, submit offer, etc. |
| Contact Reveals | request, approve, reject |
| Admin | archive, release, trust-level, quota, recalc-rating |

**How to Access Docs**:
1. **JSON endpoint**: `http://localhost:3000/api-docs/openapi.json`
2. **Interactive docs**: `http://localhost:3000/docs` → redirects to Swagger Editor
3. **Raw YAML**: `docs/specs/openapi.yaml`

**Spec Includes**:
- All request/response schemas
- Query parameters
- Response codes
- Authentication requirements

---

## MCP Server

We've built an **MCP (Model Context Protocol)** server for AI agent integration!

**Key Files**:
- `backend/mcp/src/runtime.rs` - stdio MCP sidecar runtime and tool router
- `backend/mcp/src/bin/mcp_tester.rs` - Launcher smoke test and handshake checker
- `docs/mcp/tool-catalog.md` - Tool definitions

**Status**: 
- `marketplace-mcp` sidecar **COMPILES** and the tester now passes the launcher smoke path
- Binary built: `target/debug/marketplace-mcp.exe`
- Launcher claims are passed explicitly through `MARKETPLACE_MCP_CLAIMS_JSON`
- `MARKETPLACE_MCP_DATABASE_URL` is the explicit Postgres switch for the sidecar
- The nightly smoke workflow uses the shared Rust schema bootstrap helper before the current listing create/search/get smoke path.
- The `Server Postgres` workflow runs the live Postgres integration tests against the shared schema bootstrap helper and can be rerun manually in GitHub Actions.

**MCP Tools Exposed** (11 tools — includes `agent_query`):
| Tool | Purpose | Required Role |
|------|---------|---------------|
| `create_listing` | Create seller listing | `seller_listing_writer` |
| `search_listings` | Search indexed listings | `buyer_searcher` |
| `get_listing` | Fetch one listing | authenticated client |
| `open_negotiation` | Open buyer-side negotiation | `buyer_negotiator` |
| `submit_offer` | Submit or counter offer | `buyer_negotiator` or `seller_negotiator` |
| `get_negotiation_status` | Fetch negotiation state | authorized participant |
| `accept_negotiation` | Accept a negotiation | `buyer_negotiator` or `seller_negotiator` |
| `reject_negotiation` | Reject a negotiation | `buyer_negotiator` or `seller_negotiator` |
| `request_contact_reveal` | Request contact reveal | `buyer_negotiator` |
| `approve_contact_reveal` | Seller-side approval | `seller_contact_reveal_approver` |

**How to Use with Claude Desktop**:
```json
{
  "mcpServers": {
    "marketplace": {
      "command": "path/to/marketplace-mcp.exe",
      "env": {
        "MARKETPLACE_MCP_CLAIMS_JSON": "<launcher-provided-claims-json>",
        "MARKETPLACE_MCP_DATABASE_URL": "<optional-postgres-url>"
      }
    }
  }
}
```

The MCP server uses **stdio transport** and delegates to the same `MarketplaceApp` business logic as the HTTP API.

---

## Performance Benchmarks

**Achieved**: **57,000+ ops/s** peak search throughput (11.4× above 5,000 target)

| Benchmark | Ops/Sec | vs Target (5k) |
|-----------|---------|------------------|
| Basic Search | **43,664** | **8.7×** ✅ |
| `min_seller_rating=4.0` | **41,104** | **8.2×** ✅ |
| `sort_by=rating_highest` | **42,448** | **8.5×** ✅ |
| `verified_sellers_only=true` | **40,187** | **8.0×** ✅ |
| Combined filters | **44,471** | **8.9×** ✅ |
| `seller:` prefix search | **27,803** | **5.5×** ✅ |
| `near_me=true` (no coords) | **43,752** | **8.7×** ✅ |
| Get Listing (cached) | **48,473** | **9.7×** ✅ |

### HTTP Bench Baseline (2026-05-12)

`bench_concurrent` now supports explicit claims modes:

- `public`: no auth claims header (raw transport + query throughput)
- `rotating`: authenticated requests with rotating `sub` to avoid single-bucket rate limiting
- `fixed`: authenticated requests with single fixed `sub` (expected to hit search rate limit)

| Mode | Search 100 | Search 200 | Search 500 | Notes |
|------|------------|------------|------------|-------|
| `public` | 57,733 ops/s | 57,350 ops/s | 51,569 ops/s | 0% `429` |
| `rotating` | 57,418 ops/s | 59,140 ops/s | 47,946 ops/s | 0% `429` |
| `fixed` (2k requests) | 1,765 ops/s | 0 ops/s | 0 ops/s | 97-100% `429` rate-limited |

Baseline artifacts:

- `docs/testing/benchmarks/http-bench-concurrent-public-2026-05-12.txt`
- `docs/testing/benchmarks/http-bench-concurrent-rotating-2026-05-12.txt`
- `docs/testing/benchmarks/http-bench-concurrent-fixed-2026-05-12.txt`

**Test Environment**:
- 100,160 listings (100 per seller average)
- 72,184 reviews (partial)
- 1,001 sellers
- Moka cache enabled (pre-serialized JSON)

---

## Observability Metrics (`GET /metrics`)

The server exposes Prometheus text-format metrics at `GET /metrics` (unauthenticated, no rate limit). All counters are monotonic and survive a process restart only if `restart: always` is configured externally — there is no internal persistence. The output is generated by `metrics_handler` in `backend/server/src/http/actix_runtime.rs` and reads from a live `ServerObservability` struct populated by a `wrap_fn` middleware (see `actix_runtime.rs` HttpServer factory) that calls `obs.record_request(path, status)` on every HTTP response.

### Runtime and pool (6 gauges)

| Metric | Type | Source | Meaning |
|---|---|---|---|
| `database_connections_total` | gauge | `PgPool.size()` | Max pool size (fixed at startup) |
| `database_connections_idle` | gauge | `PgPool.num_idle()` | Connections currently idle |
| `database_connections_utilization_percent` | gauge | derived | `100 * (size - idle) / size` |
| `runtime_worker_threads` | gauge | `resolve_worker_threads()` | Active tokio worker count |
| `runtime_max_worker_threads` | gauge | constant `8` | Cap (must match `resolve_worker_threads`) |
| `runtime_cpu_cores` | gauge | `num_cpus::get()` | Logical cores visible to process |

### Moka cache (8 gauges)

Listing cache: `cache_listing_entries`, `cache_listing_memory_mb`, `cache_listing_max_mb`, `cache_listing_utilization_percent`.
Search cache: `cache_search_entries`, `cache_search_memory_mb`, `cache_search_max_mb`, `cache_search_utilization_percent`.
Plus `memory_cache_total_mb` (sum of both). Memory is estimated as `entries * 3KB` (listing) and `entries * 15KB` (search) — see `metrics_handler` for the constants.

### HTTP request counters (6 counters)

All populated by `wrap_fn` calling `ServerObservability::record_request(path, status)`.

| Metric | Type | Increment condition |
|---|---|---|
| `requests_total` | counter | Every HTTP response with status < 500 (the `wrap_fn` only records on `Ok(response)`) |
| `internal_requests_total` | counter | `path.starts_with("/internal/v1/")` |
| `internal_writes_total` | counter | Internal path AND `status ∈ {200, 201, 204}` (i.e. successful admin writes) |
| `conflict_responses_total` | counter | `status == 409` |
| `quota_rejections_total` | counter | `status == 429` |
| `error_responses_total` | counter | `status >= 400` (superset of 409 and 429) |

### Ledger async-batch WAL (4 metrics, spec 0013 §4)

| Metric | Type | Source |
|---|---|---|
| `ledger_cache_hit_total` | counter | `LedgerCache::get` hit branch |
| `ledger_cache_miss_total` | counter | `LedgerCache::get` miss branch |
| `ledger_batch_lag_milliseconds` | gauge | Set by `AsyncCommitter` per flushed batch (latest value, not histogram) |
| `ledger_batch_size` | gauge | Same — entries in the most recent flushed batch |

### Alerting thresholds (suggested starting points)

These are not enforced anywhere in the server; configure them in your Prometheus / Grafana stack.

- `error_responses_total / requests_total > 0.05` sustained 5 min → page on-call (5xx rate is the leading indicator)
- `conflict_responses_total` rate > 100/s for 1 min → check for thundering-herd idempotency replays
- `quota_rejections_total` rate > 50/s for 1 min → check rate-limit policy or downstream throttle
- `ledger_cache_miss_total / (ledger_cache_hit_total + ledger_cache_miss_total) > 0.30` sustained → cache TTL may be too short or eviction too aggressive
- `ledger_batch_lag_milliseconds > 1000` for 30 s → WAL async committer falling behind, check DB write latency

### Scrape semantics

The `wrap_fn` records AFTER the handler renders its response, so a scraper's own `GET /metrics` is NOT counted in the response body it sees. The body shows the count of PRIOR requests. This is the correct Prometheus counter semantics — scrapes see prior state, not their own observation.

### Source of truth

- Format string: `backend/server/src/http/actix_runtime.rs::metrics_handler` (the `format!` literal)
- Increment logic: `backend/server/src/observability/mod.rs::ServerObservability::record_request`
- Wrap wiring: `backend/server/src/http/actix_runtime.rs::async_run` (the `wrap_fn` block in the `HttpServer::new` factory)

When you add a new field to `ServerObservabilitySnapshot`, add a matching `format!` line in `metrics_handler` AND a row to the HTTP request counters table above. Rust's compile-time arg count check will catch missing args, but won't catch the case where you add a new observability field and forget to publish it.

---

## Database Migrations: SQLx Naming Convention

All schema changes are managed via `sqlx::migrate!()` in the `bootstrap` module.
Migration files live in `backend/server/migrations/`.

### Naming Rule

Each migration file **must** follow this format:

```
{VERSION}_{description}.sql
```

- `VERSION` — sequential integer, zero-padded to 4 digits (e.g., `0001`, `0012`)
- `description` — snake_case summary of the change
- The version number is **everything before the first underscore**

### Why Split Files Don't Work

SQLx resolves the migration version by parsing the numeric prefix before the first underscore.
This means `0006_01_create_reviews_table.sql` and `0006_02_triggers.sql` **both** resolve to
version `6`, producing a duplicate primary key error at runtime.

```sql
-- ❌ BAD: both resolve to version 6
0006_01_create_reviews_table.sql
0006_02_triggers.sql

-- ✅ GOOD: each gets a unique version
0006_create_reviews_table.sql
0007_add_coordinates.sql
```

### How to Handle Helper Scripts

Scripts that are not standalone migrations (e.g., trigger setup, data seeding, manual
backfills) **must not** live in the `migrations/` directory, where `sqlx::migrate!()`
scans them at compile time. Place them in `backend/server/scripts/` instead.

```
backend/server/migrations/
├── 0001_init.sql
├── 0002_add_seller_quota_fields.sql
├── ...
├── 0006_create_reviews_table.sql        # Real migration
└── 0007_add_coordinates.sql             # Real migration

backend/server/scripts/
├── reviews_triggers.sql                 # Manual: CREATE OR REPLACE triggers
└── seed_coordinates.sql                # Manual: populates coordinates on existing data
```

### Checksum Safety

Renaming a migration file (while keeping the same content) preserves its checksum.
SQLx matches by version number and checksum, not filename — so existing databases
will not flag a checksum mismatch after a rename that preserves content.

Note that the filename is stored as `description` in `_sqlx_migrations` and is
**not** updated on rename — an existing database will retain the old filename in
the migration log. This is cosmetic only; correctness is unaffected.

---

## Production Hardening

The server includes:

1. **Tracing** (`tracing`, `tracing-actix-web`)
   - Request logging
   - Performance metrics
   - Cache hit/miss tracking

2. **Metrics** (`metrics`, `metrics-exporter-prometheus`)
   - `/metrics` endpoint (Prometheus format)
   - Custom counters and histograms

3. **Health Checks** (`/health`)
   - Deep database connectivity verification
   - Moka cache status
   - Response: `{ "status": "ok", "checks": { "database": { "status": "ok" } }`

4. **Error Handling**
   - Proper HTTP status codes
   - Structured error responses
   - Idempotency support

---

## Next Docs To Add

1. ✅ Migration strategy (0001-0007 migrations applied)
2. ✅ Environment configuration notes
3. ✅ Deployment runbook (production-ready!)
4. ✅ Benchmark runbook for `phase5_bench`
5. ✅ Local Postgres notes for benchmark runs

---

## Quick Reference

| Task | Command |
|------|---------|
| Build server | `cd backend && cargo build --release --package marketplace-server` |
| Build MCP | `cd backend && cargo build --package marketplace-mcp` |
| Run server | `./target/release/marketplace-server` |
| View API docs | `http://localhost:3000/docs` |
| Get OpenAPI JSON | `curl http://localhost:3000/api-docs/openapi.json` |
| Run benchmarks | `./target/release/bench_concurrent "http://..." 5000 "100,200,500" "rotating"` |
| Check health | `curl http://localhost:3000/health` |
| View metrics | `curl http://localhost:3000/metrics` |

---

**The server is production-ready!** 🚀

See `module-layout.md` for detailed module structure.
See `docs/specs/openapi.yaml` for complete API specification.
See `docs/mcp/tool-catalog.md` for MCP tool definitions.
