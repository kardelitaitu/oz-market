# Marketplace Server

## Overview

Production HTTP API for the marketplace. Used by the MCP sidecar, mobile apps (Android + iOS), and third-party integrations.

## Quick Start

```sh
# 1. Start Postgres
docker compose -p marketplace -f compose.postgres.yml up -d

# 2. Run migrations + populate test data
cargo run --bin bootstrap_schema

# 3. Start the server
cargo run -p oz-market-server
# Binds to 127.0.0.1:3000 by default, override with MARKETPLACE_BIND
```

## Base URLs

| Environment | URL |
|-------------|-----|
| Production | `https://api.oz-market.com` |
| Local dev | `http://127.0.0.1:3000` |
| Android emulator | `http://10.0.2.2:3000` |
| iOS simulator | `http://127.0.0.1:3000` |
| WSL2 / remote | Use host LAN IP, e.g. `http://192.168.x.x:3000` |

## Auth

The server supports two auth mechanisms, checked in order:

### 1. API Key (recommended for demos)

Set `MARKETPLACE_API_KEY` env var, then pass `x-marketplace-api-key: <key>` in requests.
Returns a full-access demo `Claims` (seller + buyer + admin).

### 2. Claims Header (dev / advanced)

Pass raw JSON `Claims` directly in the `x-marketplace-claims` header:

```json
{
  "sub": "user-1",
  "roles": ["seller_listing_writer", "buyer_searcher", "buyer_negotiator"],
  "scopes": ["listing:create", "listing:search", "listing:read", "negotiation:create", "negotiation:read", "negotiation:offer:submit", "negotiation:reveal:request", "reveal:approve"],
  "seller_account_id": "seller-1",
  "buyer_agent_id": null,
  "hardware_id": null,
  "exp": null
}
```

See `crates/auth-core/` for the `Claims` struct and role/scope definitions.

### Required roles by endpoint

| Endpoint | Required Role(s) |
|----------|------------------|
| `POST /v1/listings` | `seller_listing_writer` or `admin` |
| `GET /v1/listings/{listing_id}` | (public) |
| `GET /v1/listings/search` | (public) |
| `POST /v1/negotiations` | `buyer_negotiator` or `admin` |
| `GET /v1/negotiations/{id}` | `seller_negotiator`, `buyer_negotiator`, or `admin` |
| `POST /v1/negotiations/{id}/offers` | `buyer_negotiator`, `seller_negotiator`, or `admin` |
| `POST /v1/negotiations/{id}/accept` | `buyer_negotiator`, `seller_negotiator`, or `admin` |
| `POST /v1/negotiations/{id}/reject` | `buyer_negotiator`, `seller_negotiator`, or `admin` |
| `POST /v1/negotiations/{id}/request-contact-reveal` | `buyer_negotiator`, `seller_negotiator`, or `admin` |
| `POST /v1/contact-reveals/{id}/approve` | `seller_contact_reveal_approver` or `admin` |
| `POST /v1/agent/query` | `seller_listing_writer`, `seller_negotiator`, or `admin` |
| `GET /v1/health/agents` | (public) |
| `GET /v1/health/agents/{id}` | (public) |
| `POST /v1/health/agents/{id}/reset` | (public) |
| Internal admin endpoints | `admin` |

## Endpoints

### Public V1

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v1/listings` | Create a listing (idempotent) |
| `GET` | `/v1/listings/{listing_id}` | Get a single listing |
| `POST` | `/v1/listings/{listing_id}/reviews` | Create a review (buyer) |
| `GET` | `/v1/listings/{listing_id}/reviews` | List reviews for a listing |
| `GET` | `/v1/listings/search` | Search listings |
| `POST` | `/v1/negotiations` | Open a negotiation (idempotent) |
| `GET` | `/v1/negotiations/{negotiation_id}` | Get negotiation status |
| `POST` | `/v1/negotiations/{negotiation_id}/offers` | Submit an offer (idempotent) |
| `POST` | `/v1/negotiations/{negotiation_id}/accept` | Accept a negotiation (idempotent) |
| `POST` | `/v1/negotiations/{negotiation_id}/reject` | Reject a negotiation (idempotent) |
| `POST` | `/v1/negotiations/{negotiation_id}/request-contact-reveal` | Request contact reveal (idempotent) |
| `GET` | `/v1/events/negotiations/{negotiation_id}` | SSE stream of negotiation updates |
| `POST` | `/v1/contact-reveals/{reveal_id}/approve` | Approve contact reveal (idempotent) |
| `POST` | `/v1/agent/query` | Dispatch a query to a registered agent |

### Health & Agent Health

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/v1/health/agents` | List all agent health summaries |
| `GET` | `/v1/health/agents/{agent_id}` | Get one agent's health detail |
| `POST` | `/v1/health/agents/{agent_id}/reset` | Reset an agent's circuit breaker |

### Internal V1 (admin)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/internal/v1/listings/{listing_id}/archive` | Archive a listing |
| `POST` | `/internal/v1/reservations/{lease_id}/release` | Release a reservation |
| `PUT` | `/internal/v1/sellers/{seller_id}/trust-level` | Set seller trust level |
| `PUT` | `/internal/v1/sellers/{seller_id}/quota-override` | Override seller quota |
| `POST` | `/internal/v1/sellers/{seller_id}/recalculate-rating` | Recalculate seller rating |
| `POST` | `/internal/v1/sellers/{seller_id}/credits` | Adjust seller credits |
| `POST` | `/internal/v1/reviews/{review_id}/approve` | Approve a review |
| `POST` | `/internal/v1/reviews/{review_id}/reject` | Reject a review |
| `GET` | `/internal/v1/rate-limits` | Snapshot of rate-limiter buckets |

### Observability

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check (no auth required) |
| `GET` | `/metrics` | Prometheus-format metrics |
| `GET` | `/docs` | Swagger Editor (interactive docs) |

## Idempotency

All write endpoints (`create_listing`, `open_negotiation`, `submit_offer`, `accept_negotiation`, `reject_negotiation`, `request_contact_reveal`, `approve_contact_reveal`) require an `idempotency_key`.

- Keys expire after 24 hours
- Same key + same request body → returns original response (safe retry)
- Same key + different body → `409 Conflict`
- Persisted in Postgres (survives server restarts)

## Rate Limits

All rate limits use a per-subject sliding window (60-second window):

| Action | Limit | Per |
|--------|-------|-----|
| Search | 60/min | Per `sub` |
| Create listing | 10/min | Per `sub` |
| Open negotiation / offer / accept / reject | 20/min | Per `sub` |
| Contact reveal request / approve | 10/min | Per `sub` |
| Agent query | 20/min | Per `sub` |
| New seller (daily) | 3/day | Per `sub` |
| New seller (hourly) | 1/hour | Per `sub` |

Rate-limit headers (`X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`) are returned on every response.

## Error Responses

All errors return JSON in this shape:

```json
{
  "error": {
    "code": "not_found",
    "message": "listing not found"
  }
}
```

| HTTP Status | Code | Meaning |
|-------------|------|---------|
| 400 | `invalid_field` | Validation error |
| 401 | `unauthorized` | Missing or invalid credentials |
| 403 | `forbidden` | Missing role or scope |
| 404 | `not_found` | Resource not found |
| 409 | `conflict` | Idempotency key conflict or state conflict |
| 429 | `rate_limited` | Rate limit hit |
| 500 | `internal_error` | Server error |

## Configuration

| Env Var | Default | Description |
|---------|---------|-------------|
| `DATABASE_URL` | — | Postgres connection string (required) |
| `DATABASE_MAX_CONNECTIONS` | `200` | Postgres pool size |
| `MARKETPLACE_BIND` | `127.0.0.1:3000` | Bind address |
| `MARKETPLACE_CACHE_ENABLED` | `true` | Enable in-memory listing/search cache |
| `MARKETPLACE_API_KEY` | — | Shared API key for zero-config auth (server + MCP) |
| `TOKIO_WORKER_THREADS` | `auto` | Tokio async worker threads (auto: num_cpus-1, cap 8) |
| `ACTIX_WORKERS` | `auto` | Actix-web HTTP worker threads (auto: num_cpus*4, 16-64) |
| `SHUTDOWN_TIMEOUT_SECS` | `30` | Graceful shutdown timeout in seconds |
| `LOG_FORMAT` | `plain` | Log format: `json` for structured JSON output |
| `LISTING_CACHE_MAX_MB` | `200` | Max memory for listing cache in MB |
| `SEARCH_CACHE_MAX_MB` | `100` | Max memory for search cache in MB |
| `LEDGER_CACHE_TTL_SECS` | `3600` | Credit ledger cache TTL in seconds |
| `LEDGER_WAL_PATH` | `./data/wal/` | Write-ahead log directory for crash recovery |

## Project Layout (relevant to mobile devs)

| Path | Purpose |
|------|---------|
| `docs/specs/openapi.yaml` | Canonical API contract (source of truth for shapes) |
| `crates/api-contract/` | Shared Rust types matching the OpenAPI spec |
| `tests/postgres_flows.rs` | Postgres integration tests (reference for flows) |
| `src/app.rs` | Core `MarketplaceApp` — all business logic |
| `src/http/handlers.rs` | Shared handler utilities (error types, idempotency helpers) |

## OpenAPI Spec

The frozen V1 contract lives at `docs/specs/openapi.yaml` (~20 endpoints, 20+ schemas). Mobile clients should use this as the source of truth for request/response shapes.
