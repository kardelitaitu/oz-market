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
cargo run -p marketplace-server
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

## Auth (JWT)

All authenticated endpoints require a JWT in the `x-marketplace-claims` header (dev) or `Authorization: Bearer <token>` header (production).

### Claims structure

```json
{
  "sub": "user-1",
  "roles": ["seller_listing_writer", "buyer_searcher", "buyer_negotiator"],
  "scopes": ["listings:create", "listings:search", "negotiations:write"],
  "seller_account_id": "seller-1",
  "buyer_agent_id": null,
  "hardware_id": null,
  "exp": null
}
```

### Dev auth

For local development, pass claims as a base64-encoded JSON in the `x-marketplace-claims` header. See `crates/auth-core/` for `Claims` struct and signing logic.

### Required roles by endpoint

| Endpoint | Required Role |
|----------|---------------|
| `POST /v1/listings` | `seller_listing_writer` |
| `GET /v1/listings/search` | `buyer_searcher` |
| `POST /v1/negotiations` | `buyer_negotiator` |
| `POST /v1/negotiations/{id}/offers` | `buyer_negotiator` or `seller_negotiator` |
| `POST /v1/negotiations/{id}/accept` | `buyer_negotiator` or `seller_negotiator` |
| `POST /v1/negotiations/{id}/reject` | `buyer_negotiator` or `seller_negotiator` |
| `POST /v1/negotiations/{id}/request-contact-reveal` | `buyer_negotiator` |
| `POST /v1/contact-reveals/{id}/approve` | `seller_contact_reveal_approver` |
| Admin endpoints | `admin` |

## Endpoints

### Public V1

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v1/listings` | Create a listing (idempotent) |
| `GET` | `/v1/listings/{listing_id}` | Get a single listing |
| `GET` | `/v1/listings/search?query=&category=&sort_by=&limit=` | Search listings |
| `POST` | `/v1/negotiations` | Open a negotiation (idempotent) |
| `GET` | `/v1/negotiations/{negotiation_id}` | Get negotiation status |
| `POST` | `/v1/negotiations/{negotiation_id}/offers` | Submit an offer (idempotent) |
| `POST` | `/v1/negotiations/{negotiation_id}/accept` | Accept a negotiation (idempotent) |
| `POST` | `/v1/negotiations/{negotiation_id}/reject` | Reject a negotiation (idempotent) |
| `POST` | `/v1/negotiations/{negotiation_id}/request-contact-reveal` | Request contact reveal (idempotent) |
| `POST` | `/v1/contact-reveals/{reveal_id}/approve` | Approve contact reveal (idempotent) |

### Internal V1 (admin)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/internal/v1/listings/{listing_id}/archive` | Archive a listing |
| `POST` | `/internal/v1/reservations/{lease_id}/release` | Release a reservation |
| `PUT` | `/internal/v1/sellers/{seller_id}/trust-level` | Set seller trust level |
| `PUT` | `/internal/v1/sellers/{seller_id}/quota-override` | Override seller quota |

### Observability

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check (no auth required) |
| `GET` | `/metrics` | Prometheus-format metrics |

## Idempotency

All write endpoints (`create_listing`, `open_negotiation`, `submit_offer`, `accept_negotiation`, `reject_negotiation`, `request_contact_reveal`, `approve_contact_reveal`) require an `idempotency_key`.

- Keys expire after 24 hours
- Same key + same request body → returns original response (safe retry)
- Same key + different body → `409 Conflict`
- Persisted in Postgres (survives server restarts)

## Rate Limits (search)

| Window | Limit | Scope |
|--------|-------|-------|
| 60 seconds | 60 requests | Per-IP / per-claims |
| 24 hours | 3000 requests | Per-token |

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
| 403 | `forbidden` | Missing role or scope |
| 404 | `not_found` | Resource not found |
| 409 | `conflict` | Idempotency key conflict or state conflict |
| 429 | `quota_exceeded` | Rate limit hit |
| 500 | `internal_error` | Server error |

## Configuration

| Env Var | Default | Description |
|---------|---------|-------------|
| `MARKETPLACE_BIND` | `127.0.0.1:3000` | Bind address |
| `DATABASE_URL` | — | Postgres connection string |
| `DATABASE_MAX_CONNECTIONS` | `100` | Postgres pool size |
| `TOKIO_WORKER_THREADS` | `num_cpus - 1` | Async worker threads |

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
