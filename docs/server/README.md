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
