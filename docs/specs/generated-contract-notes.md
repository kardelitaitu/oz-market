# Generated Client/Server Contract Notes

Documentation for generating client and server contracts from the frozen OpenAPI specification.

## Overview

The project uses a **spec-first** approach with a frozen OpenAPI 3.1 specification at `docs/specs/openapi.yaml`.

**Goal**: Generate type-safe clients and server stubs from the canonical spec.

---

## Source of Truth

- **Frozen Spec**: `docs/specs/openapi.yaml`
- **Rust Types**: `backend/crates/api-contract/src/` (implements the spec with `utoipa` ToSchema)
- **Internal API**: `docs/specs/internal-api-spec.md` (separate internal surface)

---

## Available Generators

### 1. OpenAPI Generator (Recommended)

**Tool**: [OpenAPI Generator](https://openapi-generator.tech/)

**Installation**:
```bash
# macOS
brew install openapi-generator

# Linux/Windows (Java required)
# Download the JAR from: https://repo1.maven.org/maven2/org/openapitools/openapi-generator-cli/
```

**Generate Rust Client** (for mobile/desktop apps):
```bash
openapi-generator generate \
  -i docs/specs/openapi.yaml \
  -g rust \
  -o generated/rust-client \
  --additional-properties=packageName=marketplace-client
```

**Generate TypeScript Client** (for web frontends):
```bash
openapi-generator generate \
  -i docs/specs/openapi.yaml \
  -g typescript-fetch \
  -o generated/typescript-client \
  --additional-properties=npmName=marketplace-client
```

**Generate Server Stub** (for new server implementations):
```bash
openapi-generator generate \
  -i docs/specs/openapi.yaml \
  -g rust-axum \
  -o generated/rust-server \
  --additional-properties=serverPort=3000
```

---

### 2. `utoipa-gen` (Rust-Specific)

Since we already use `utoipa` in `api-contract`, we can generate specs from code instead of the frozen YAML.

**Note**: This conflicts with our "frozen spec" policy. Only use if spec-first policy changes.

```bash
# In backend/crates/api-contract/
cargo run --example generate_openapi
```

---

### 3. `swagger-codegen` (Alternative)

**Tool**: Part of Swagger ecosystem

```bash
# Generate Python client
swagger-codegen generate \
  -i docs/specs/openapi.yaml \
  -l python \
  -o generated/python-client
```

---

## Integration with Existing Code

### Rust `api-contract` Crate

Our `backend/crates/api-contract/src/` already contains:
- 21 types with `utoipa::ToSchema` derives
- Proper serde serialization matching the OpenAPI spec
- Validation functions for string patterns

**How generated clients should map to our Rust types**:

| OpenAPI Component | Rust Type in `api-contract` |
|-------------------|-------------------------------|
| `ResourceId` | `ResourceId` (type alias: String) |
| `Category` | `Category` (enum) |
| `ListingPayload` | `ListingPayload` (struct) |
| `CreateListingRequest` | `CreateListingRequest` (struct) |
| `NegotiationResponse` | `NegotiationResponse` (struct) |
| `ApiErrorResponse` | `ApiErrorResponse` (struct) |

---

## Generation Workflow

### For Mobile Clients (Android/iOS)

#### Step 1: Generate Base Client

```bash
# Generate Kotlin client for Android
openapi-generator generate \
  -i docs/specs/openapi.yaml \
  -g kotlin \
  -o mobile/app-android/libs/client/

# Generate Swift client for iOS
openapi-generator generate \
  -i docs/specs/openapi.yaml \
  -g swift \
  -o mobile/app-ios/Sources/Client/
```

#### Step 2: Customize Authentication

Both generated clients need to:
1. Add bearer token handling (using `X-Marketplace-Claims` header for testing)
2. Implement token refresh logic
3. Match the `idempotency_key` pattern for retries

#### Step 3: Align with Contract Rules

From `docs/whitepaper/10-api-contract.md`:
- **Same payload fields** across HTTP, MCP, and mobile
- **No alternate field shapes** for different clients
- **Use `idempotency_key`** on all state-creating writes

---

### For Server Implementation

#### Option A: Generate Server Stub (Not Recommended)

```bash
openapi-generator generate \
  -i docs/specs/openapi.yaml \
  -g rust-axum \
  -o generated/server-stub
```

**Why not recommended**:
- We already have a custom TCP runtime in `backend/server/src/http/runtime.rs`
- Our server reuses business logic via `MarketplaceApp` shared layer
- Generated stubs would conflict with our architecture

#### Option B: Use `api-contract` as Contract (Recommended)

Our existing `api-contract` crate **is** the server contract:
- Already has all types with `ToSchema`
- Already used by `server` and `mcp` crates
- Already validated against frozen spec (via CI checks)

**No generation needed** - the Rust types are the contract!

---

## MCP Tool Mapping

Our MCP integration already uses the same contract:

| MCP Tool | HTTP Equivalent | Rust Type |
|-----------|----------------|----------|
| `create_listing` | `POST /v1/listings` | `CreateListingRequest` → `CreateListingResponse` |
| `search_listings` | `GET /v1/listings/search` | `SearchRequest` → `SearchResponse` |
| `open_negotiation` | `POST /v1/negotiations` | `OpenNegotiationRequest` → `NegotiationResponse` |
| `get_negotiation_status` | `GET /v1/negotiations/{id}` | `NegotiationResponse` |
| `request_contact_reveal` | `POST /v1/.../request-contact-reveal` | `RequestContactRevealRequest` → `ContactRevealResponse` |

**No generation needed** - MCP already delegates to `MarketplaceApp` which uses the same types.

---

## CI Integration

### Spec Validation + Client Generation

Create `.github/workflows/contract-check.yml`:

```yaml
name: Contract Check

on:
  pull_request:
    paths:
      - 'docs/specs/openapi.yaml'
      - 'backend/crates/api-contract/**'

jobs:
  validate-spec:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Validate OpenAPI spec
        run: |
          # Install spectral
          npm install -g @stoplight/spectral-cli
          spectral lint docs/specs/openapi.yaml
      
      - name: Check for breaking changes
        run: |
          # Install oasdiff
          # ... (see docs/specs/ci-commands.md)
          oasdiff diff docs/specs/baseline/openapi.yaml docs/specs/openapi.yaml
      
      - name: Generate Rust client (verify it compiles)
        run: |
          openapi-generator generate \
            -i docs/specs/openapi.yaml \
            -g rust \
            -o /tmp/rust-client
          cd /tmp/rust-client && cargo check
```

---

## Contract Versioning

### Schema Version

The API uses `schema_version: "1.0"` in `ListingPayload`.

**Rules** (from `docs/whitepaper/10-api-contract.md`):
- **Breaking changes** require new `schema_version`
- **Optional fields** can be added without breaking old agents
- **All changes** must update the frozen spec first

### When to Generate New Clients

1. **Non-breaking change** (add optional field):
   - Update `openapi.yaml`
   - Regenerate clients (they'll have new optional fields)
   - Old clients still work

2. **Breaking change** (new required field):
   - Increment `schema_version` to `"1.1"`
   - Create new client version
   - Keep old client for backward compatibility

---

## Cross-Reference

| Document | Purpose |
|-----------|---------|
| `docs/specs/openapi.yaml` | Frozen OpenAPI 3.1 specification |
| `docs/specs/README.md` | Spec docs index (this item is #2) |
| `docs/specs/schemas/README.md` | Schema component definitions |
| `docs/specs/internal-api-spec.md` | Internal `/internal/v1` spec |
| `docs/whitepaper/10-api-contract.md` | Contract rules and transport mapping |
| `backend/crates/api-contract/src/` | Rust types with `utoipa::ToSchema` |

---

## Recommended Workflow

### For Developers Adding New Endpoints:

1. **Update frozen spec**: Edit `docs/specs/openapi.yaml`
2. **Update Rust types**: Add to `backend/crates/api-contract/src/`
3. **Run CI checks**: Spectral, Redocly, oasdiff
4. **Generate clients** (if needed): `openapi-generator generate ...`
5. **Test**: `cd backend && cargo test`

### For Mobile Teams:

1. **Pull latest spec**: `git pull origin main`
2. **Generate client**: `openapi-generator generate -i docs/specs/openapi.yaml -g kotlin -o mobile/app-android/libs/client/`
3. **Implement UI**: Use generated types (they match the contract!)
4. **Test against server**: Use the same `idempotency_key` pattern

---

## Common Generation Options

### OpenAPI Generator Useful Flags

```bash
# Skip validating spec (if you already do it in CI)
--skip-validate-spec

# Custom package name
--additional-properties=packageName=marketplace-api

# Use async/await (Rust)
--additional-properties=useSingleRequestParameter=false,supportAsync=true

# Generate model tests
--additional-properties=generateModelTests=true,generateApiTests=true
```

---

## Notes on Our Architecture

### Why We Don't Generate Server Code

- **Custom TCP runtime**: We use manual HTTP parsing in `runtime.rs`
- **Shared business logic**: `MarketplaceApp` wraps repos + services
- **MCP integration**: Already shares the same types via `api-contract`
- **Frozen spec policy**: Generation from code would violate this

### What We Do Generate

- **Client libraries** for mobile (Android/iOS)
- **TypeScript client** (if web frontend comes later)
- **Documentation** (Redocly already set up)

---

## Next Steps (from `docs/specs/README.md`)

This document fulfills item #2 from `docs/specs/README.md`:
- ✅ Generated client/server contract notes (this document)
- Next: Item #4: CI workflow file when automation starts
- Next: Item #5: Any `/internal/v1` policy docs if needed

---

## Maintenance

When the spec changes:
1. Update `docs/specs/openapi.yaml`
2. Update `backend/crates/api-contract/` types (with `ToSchema`)
3. Regenerate clients: `openapi-generator generate ...`
4. Update this document if generation steps change
5. Run `cargo test` to verify everything still works

---

**Last Updated**: 2026-05-06 (session: Item #2 - Generated client/server contract notes)
**Status**: ✅ Complete for documentation, ⚠️ Client generation is on-demand (when mobile teams need it)
