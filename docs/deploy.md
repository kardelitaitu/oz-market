# Deployment Guide

## Prerequisites

- Linux server (recommended: 2 vCPU, 4GB RAM — Hetzner CCX12, DigitalOcean $12 droplet, or similar)
- Docker + Docker Compose (recommended), or Rust toolchain for manual builds
- Domain name (optional, for Swagger docs)

## Quick Deploy (Docker Compose)

```bash
# Clone the repo on your server
git clone <your-repo-url> project-the-marketplace
cd project-the-marketplace

# Set a secure API key for demo auth
export MARKETPLACE_API_KEY=$(openssl rand -hex 32)

# Start everything
docker compose up -d

# Check logs
docker compose logs -f server
```

The server starts on port 3000 with PostgreSQL auto-configured. Migrations run automatically on first boot.

## Manual Deploy (No Docker)

### 1. Install PostgreSQL

```bash
# Debian/Ubuntu
sudo apt update && sudo apt install -y postgresql postgresql-client
sudo systemctl start postgresql

# Create database and user
sudo -u postgres psql -c "CREATE USER marketplace WITH PASSWORD 'your-password';"
sudo -u postgres psql -c "CREATE DATABASE marketplace OWNER marketplace;"
```

### 2. Build and Run

```bash
# Build the release binary (from project root)
cd backend
cargo build --release --package marketplace-server

# Set environment
export DATABASE_URL=postgres://marketplace:your-password@localhost:5432/marketplace
export MARKETPLACE_API_KEY=your-demo-key
export MARKETPLACE_BIND=0.0.0.0:3000

# Run (migrations auto-apply)
./target/release/marketplace-server
```

## Configuration Reference

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | (required) | PostgreSQL connection string |
| `MARKETPLACE_BIND` | `127.0.0.1:3000` | Server listen address |
| `MARKETPLACE_API_KEY` | (none) | If set, enables API key auth via `x-marketplace-api-key` header |
| `MARKETPLACE_DISABLE_CACHE` | `false` | Set to `1` to disable in-memory caching |
| `LISTING_CACHE_MAX_MB` | `200` | Listing cache memory limit |
| `SEARCH_CACHE_MAX_MB` | `100` | Search cache memory limit |
| `RUST_LOG` | `info` | Log level (debug, info, warn, error) |

## Auth Methods

The server supports two auth mechanisms, checked in order:

### 1. API Key (recommended for demos)

```bash
export MARKETPLACE_API_KEY=your-secret-key
```

Then any request with the matching header gets full access:

```bash
curl -H 'x-marketplace-api-key: your-secret-key' http://your-server:3000/v1/listings/search
```

### 2. Claims Header (advanced)

Pass raw JSON Claims directly:

```bash
curl -H 'x-marketplace-claims: {"sub":"seller-1","roles":["seller_listing_writer"],"scopes":["listing:create","listing:read","listing:search"],"seller_account_id":"seller-1"}' http://your-server:3000/v1/listings
```

## Verification

### Health check

```bash
curl http://localhost:3000/health
# {"status":"ok","checks":{"database":{"status":"ok"},"cache":{"status":"ok"}}}
```

### API Docs

```bash
# Swagger docs (uses Host header for dynamic URL)
curl http://localhost:3000/docs

# Raw OpenAPI spec
curl http://localhost:3000/api-docs/openapi.json
```

### Metrics

```bash
curl http://localhost:3000/metrics
```

## Demo Transaction Flow

Run these commands against your deployed server to prove the full agent-to-agent transaction cycle.

### Setup

```bash
BASE=http://localhost:3000
AUTH=-H 'x-marketplace-api-key: your-secret-key'
NOW=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
```

### Step 1: Create a listing (seller)

```bash
SELLER_RESP=$(curl -s -X POST $BASE/v1/listings $AUTH \
  -H 'Content-Type: application/json' \
  -d '{
    "idempotency_key": "demo-create-1",
    "listing": {
      "schema_version": "1.0",
      "owner_id": "demo-seller",
      "listing_type": "product",
      "category": "laptop",
      "title": "ThinkPad T480",
      "condition": "used",
      "price": { "currency": "USD", "amount": 450.00 },
      "location": {
        "country_code": "JP",
        "country_name": "Japan",
        "city": "Osaka"
      },
      "description": "Good battery health, no major scratches"
    }
  }')
echo "$SELLER_RESP" | jq .
LISTING_ID=$(echo "$SELLER_RESP" | jq -r '.listing_id')
```

### Step 2: Search for the listing (buyer agent)

```bash
curl -s "$BASE/v1/listings/search?query=ThinkPad" $AUTH | jq .
```

### Step 3: Open a negotiation (buyer)

```bash
NEG_RESP=$(curl -s -X POST $BASE/v1/negotiations $AUTH \
  -H 'Content-Type: application/json' \
  -d "{
    \"listing_id\": \"$LISTING_ID\",
    \"buyer_agent_id\": \"demo-buyer\",
    \"offer_currency\": \"USD\",
    \"offer_amount\": 400.00,
    \"idempotency_key\": \"demo-neg-1\"
  }")
echo "$NEG_RESP" | jq .
NEG_ID=$(echo "$NEG_RESP" | jq -r '.negotiation_id')
```

### Step 4: Submit a counter-offer (seller)

```bash
curl -s -X POST "$BASE/v1/negotiations/$NEG_ID/offers" $AUTH \
  -H 'Content-Type: application/json' \
  -d '{
    "offer_currency": "USD",
    "offer_amount": 425.00,
    "idempotency_key": "demo-offer-1"
  }' | jq .
```

### Step 5: Accept the negotiation (buyer)

```bash
curl -s -X POST "$BASE/v1/negotiations/$NEG_ID/accept" $AUTH \
  -H 'Content-Type: application/json' \
  -d '{
    "idempotency_key": "demo-accept-1"
  }' | jq .
```

### Step 6: Request contact reveal (buyer)

```bash
REVEAL_RESP=$(curl -s -X POST "$BASE/v1/negotiations/$NEG_ID/request-contact-reveal" $AUTH \
  -H 'Content-Type: application/json' \
  -d '{
    "idempotency_key": "demo-reveal-1"
  }')
echo "$REVEAL_RESP" | jq .
REVEAL_ID=$(echo "$REVEAL_RESP" | jq -r '.reveal_id')
```

### Step 7: Approve contact reveal (seller)

```bash
curl -s -X POST "$BASE/v1/contact-reveals/$REVEAL_ID/approve" $AUTH \
  -H 'Content-Type: application/json' \
  -d '{}' | jq .
```

### Step 8: Check negotiation status (any participant)

```bash
curl -s "$BASE/v1/negotiations/$NEG_ID" $AUTH | jq .
```

### Expected outcome

After step 8, the negotiation status should be `accepted` with a `contact_phone` field containing the seller's revealed phone number. This proves the full agent-to-agent transaction lifecycle.

## Architecture Notes

- The server runs the Actix-web HTTP runtime (not the TCP runtime).
- Migrations apply automatically on startup — no manual step needed.
- API key auth (`MARKETPLACE_API_KEY`) maps to full-access demo Claims for HTTP. (The MCP server reads its own `MARKETPLACE_MCP_CLAIMS_JSON` / `MARKETPLACE_MCP_ALLOW_DEV_CLAIMS` env vars; the API key is not consulted there.)
- Caches are sized for a 4GB VPS (200MB listing, 100MB search).

## MCP Server (Desktop Agent Integration)

The MCP server lets desktop AI agents (Claude Desktop, Cursor, etc.) interact with the marketplace directly.

### Running locally

```bash
# Build the MCP binary
cd backend
cargo build --release --package marketplace-mcp

# Run with dev claims (in-memory storage, ephemeral)
MARKETPLACE_MCP_ALLOW_DEV_CLAIMS=1 ./target/release/marketplace-mcp
```

### Running against the deployed database

```bash
MARKETPLACE_MCP_DATABASE_URL=postgres://marketplace:password@your-server:5432/marketplace \
  MARKETPLACE_API_KEY=your-key \
  ./target/release/marketplace-mcp
```

The MCP server supports the same `MARKETPLACE_API_KEY` env var as the HTTP server. Falls back to `MARKETPLACE_MCP_CLAIMS_JSON` or `MARKETPLACE_MCP_ALLOW_DEV_CLAIMS=1` for local dev.

### Connecting Claude Desktop

Add to your Claude Desktop configuration (`settings.json`):

```json
{
  "mcpServers": {
    "marketplace": {
      "command": "/path/to/marketplace-mcp",
      "env": {
        "MARKETPLACE_MCP_DATABASE_URL": "postgres://marketplace:password@your-server:5432/marketplace",
        "MARKETPLACE_API_KEY": "your-key"
      }
    }
  }
}
```

### Available MCP Tools (10 tools)

| Tool | Description |
|------|-------------|
| `create_listing` | Create a new listing |
| `search_listings` | Search listings |
| `get_listing` | Get a listing by id |
| `open_negotiation` | Open a negotiation |
| `submit_offer` | Submit an offer |
| `accept_negotiation` | Accept a negotiation |
| `reject_negotiation` | Reject a negotiation |
| `get_negotiation_status` | Get negotiation status |
| `request_contact_reveal` | Request contact reveal |
| `approve_contact_reveal` | Approve a contact reveal |

### Verification

```bash
# Run the MCP smoke test suite
cd backend
MARKETPLACE_MCP_ALLOW_DEV_CLAIMS=1 cargo run --release --package marketplace-mcp --bin mcp_tester
```

Expected output: all 6 tests pass (initialize, list tools, create listing, search, get listing, get non-existent).
