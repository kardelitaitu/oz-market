# Specs Documentation

This folder is for implementation-ready specifications.

## Intended Contents

- `openapi.yaml` - **✅ COMPLETE** (20+ endpoints documented)
- `internal-api-outline.md`
- `validation-checklist.md`
- `spectral-rules.md`
- `redocly-notes.md`
- `ci-commands.md` - **✅ EXISTS**
- `yamllint.yaml`
- `run-oasdiff-breaking.ps1`
- `.spectral.yaml`
- `redocly.yaml`
- schema component definitions - **✅ EXISTS** (see `schemas/README.md`)
- request/response validation notes - **✅ EXISTS**
- generated client/server contract artifacts - **✅ EXISTS** (see `generated-contract-notes.md`)

## Current Status (Updated 2026-05-08)

| Item | Status | Details |
|------|--------|---------|
| **OpenAPI Spec** | ✅ **COMPLETE** | 30+ endpoints, all request/response schemas |
| **Interactive Docs** | ✅ **LIVE** | Swagger Editor at `http://localhost:3000/docs` |
| **JSON Endpoint** | ✅ **LIVE** | `http://localhost:3000/api-docs/openapi.json` |
| **Internal API Spec** | ✅ **EXISTS** | `internal-api-spec.md` (comprehensive) |
| **Generated Contract Notes** | ✅ **EXISTS** | `generated-contract-notes.md` |
| **CI Commands** | ✅ **EXISTS** | `ci-commands.md` |
| **Schema Components** | ✅ **DOCUMENTED** | See `schemas/README.md` |

## OpenAPI Specification Details

**File**: `openapi.yaml`

**Endpoints Documented** (30+ total):

### Listings & Reviews (5 endpoints)
- `POST /v1/listings` - Create listing
- `GET /v1/listings/{listing_id}` - Get listing
- `GET /v1/listings/search` - Search listings
- `POST /v1/listings/{listing_id}/reviews` - Create review
- `GET /v1/listings/{listing_id}/reviews` - List reviews

### Negotiations & Events (7 endpoints)
- `POST /v1/negotiations` - Open negotiation
- `GET /v1/negotiations/{negotiation_id}` - Get status
- `POST /v1/negotiations/{negotiation_id}/offers` - Submit offer
- `POST /v1/negotiations/{negotiation_id}/accept` - Accept negotiation
- `POST /v1/negotiations/{negotiation_id}/reject` - Reject negotiation
- `POST /v1/negotiations/{negotiation_id}/request-contact-reveal` - Request reveal
- `GET /v1/events/negotiations/{negotiation_id}` - SSE stream

### Contact Reveals (1 endpoint)
- `POST /v1/contact-reveals/{reveal_id}/approve` - Approve

### Agent & Health (5 endpoints)
- `POST /v1/agent/query` - Dispatch agent query
- `GET /v1/health/agents` - List agent health
- `GET /v1/health/agents/{agent_id}` - Get agent health detail
- `POST /v1/health/agents/{agent_id}/reset` - Reset circuit breaker
- `GET /internal/v1/rate-limits` - Rate limiter snapshot

### Admin (8 endpoints)
- `POST /internal/v1/listings/{listing_id}/archive` - Archive listing
- `POST /internal/v1/reservations/{lease_id}/release` - Release reservation
- `PUT /internal/v1/sellers/{seller_id}/trust-level` - Set trust level
- `PUT /internal/v1/sellers/{seller_id}/quota-override` - Set quota override
- `POST /internal/v1/sellers/{seller_id}/recalculate-rating` - Recalculate rating
- `POST /internal/v1/sellers/{seller_id}/credits` - Adjust credits
- `POST /internal/v1/reviews/{review_id}/approve` - Approve review
- `POST /internal/v1/reviews/{review_id}/reject` - Reject review

## How to Use the OpenAPI Spec

### 1. View Interactive Documentation
```bash
# Start server
cd backend && cargo run --release --package marketplace-server

# Open in browser
http://localhost:3000/docs
```

### 2. Get Raw JSON
```bash
curl http://localhost:3000/api-docs/openapi.json
```

### 3. Validate Spec
```bash
# Using Spectral (if installed)
spectral lint docs/specs/openapi.yaml

# Using Redocly (if installed)
redocly lint docs/specs/openapi.yaml
```

### 4. Generate Client Code
```bash
# Using openapi-generator
openapi-generator generate \
  -i docs/specs/openapi.yaml \
  -g typescript-fetch \
  -o frontend/src/api-client
```

## Next Docs To Add

1. ✅ shared schema component definitions (see `schemas/README.md`)
2. ✅ generated client/server contract notes (see `generated-contract-notes.md`)
3. ✅ internal `/internal/v1` spec (see `internal-api-spec.md`)
4. 🔜 **CI/CD pipeline** - automate OpenAPI validation in PRs
5. 🔜 **Contract testing** - ensure server matches spec
6. 🔜 **Breaking change detection** - `oasdiff` in CI

## File Inventory

| File | Status | Description |
|------|--------|-------------|
| `openapi.yaml` | ✅ Complete | 20+ endpoints, full schemas |
| `internal-api-outline.md` | ✅ Exists | Internal API plan |
| `internal-api-spec.md` | ✅ Exists | Detailed internal API spec |
| `generated-contract-notes.md` | ✅ Exists | Notes on generated code |
| `ci-commands.md` | ✅ Exists | CI validation commands |
| `schemas/README.md` | ✅ Exists | Schema component docs |
| `spectral-rules.md` | ⚠️ TODO | Custom Spectral rules |
| `.spectral.yaml` | ⚠️ TODO | Spectral config |

## Specification Library

Specifications are stored under `_active/` (in progress) and `_done/` (completed).

### Active Specs

| ID | Title | Priority |
|----|-------|----------|
| 0018 | Update Affected Documents | P3 |

### Completed Specs

| ID | Title | Priority |
|----|-------|----------|
| 0001 | Unified Listings Endpoint | P1 |
| 0002 | Listing ID Cleanup | P2 |
| 0003 | Negotiation Offer History | P1 |
| 0004 | HTTP Benchmark Stability | P3 |
| 0005 | Negotiation Hardening And Parity | P1 |
| 0010 | Credit Ledger Schema & Domain | P1 |
| 0011 | Dual-Layer Ledger Cache | P1 |
| 0012 | Ledger Cache Invalidation | P1 |
| 0013 | Async Batch WAL + Committer | P1 |
| 0014 | Agent Routing And Dispatch Core | P2 |
| 0015 | Agent Metrics Collector | P2 |
| 0016 | Predictive Latency Scoring | P2 |
| 0017 | Agent Circuit-Breaker And Health API | P2 |

---

**The API is fully documented!** 🎉

See `../server/README.md` for server documentation.
See `openapi.yaml` for complete API specification.
