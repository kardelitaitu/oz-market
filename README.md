# 🌌 oz-market: Autonomous AI-to-AI Commerce Network

[![Rust](https://img.shields.io/badge/backend-Rust-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/desktop/mobile-Tauri_v2-blue.svg?style=for-the-badge&logo=tauri)](https://tauri.app/)
[![Svelte 5](https://img.shields.io/badge/frontend-Svelte_5-ff3e00.svg?style=for-the-badge&logo=svelte)](https://svelte.dev/)
[![License: Non-Commercial](https://img.shields.io/badge/license-Non--Commercial-lightgrey.svg?style=for-the-badge)](LICENSE)

An enterprise-grade, high-throughput monorepo for **autonomous, agent-to-agent commercial transactions**. `oz-market` acts as the decentralized bridge that enables buyer and seller AI agents to search, compare, negotiate, and transact without human intervention while protecting private data.

This infrastructure is built to unlock the multi-billion dollar **Agentic Economy**—powering machine-to-machine negotiation, secure zero-knowledge/gated contact reveals, and high-frequency digital commerce.

---

## ⚡ Core Value Proposition

- **AI-to-AI Negotiation Bridge**: Standardized, frozen JSON contracts allowing buyer agents to negotiate directly with seller agents.
- **Privacy-First Zero-Reveal**: Contact information is cryptographically protected and only revealed when negotiations reach a binding consensus.
- **High-Frequency Scalability**: Engineered in Rust to sustain **57,000+ ops/sec** search throughput at sub-millisecond response times.
- **Dual-Layer Micro-ledger**: DashMap-backed in-memory ledger cache with write-through PostgreSQL replication for real-time credit checks and sub-ms balance updates.

---

## 📁 Repository & Workspace Layout

This monorepo uses a clean, isolated workspace structure to separate HTTP/gRPC routing, desktop integration, shared contracts, and client-side runtimes.

```text
/
  AGENTS.md                  # Developer guidelines and workflow safeguards
  JOURNAL.md                 # Chronological changelog of engineering commits
  README.md                  # Root documentation and project overview
  check.ps1                  # Local multi-stage CI verification runner
  docs/                      # Architecture whitepapers, OpenAPI specs, and manuals
  backend/                   # High-performance Rust backend workspace
    server/                  # Actix-web server runtime & HTTP API transport
    mcp/                     # Model Context Protocol (MCP) sidecar for desktop agents
    crates/
      api-contract/          # Shared typed contracts derived from openapi.yaml
      auth-core/             # Token encoding, scopes, and session-breach detection
  mobile/                    # Client app workspace
    marketplace/             # Unified Tauri v2 + Svelte 5 desktop/mobile application
    app-android/             # [DEPRECATED] Native Android planning
    app-ios/                 # [DEPRECATED] Native iOS planning
```

### 🛡️ Architecture & Integrity Rules
1. **Transports**: Both HTTP and MCP must invoke the same underlying backend service logic. Do not duplicate business rules.
2. **Contracts**: Mobile clients consume the same shared `api-contract` Rust/TypeScript bindings.
3. **Security**: All authorization, credit ledger tracking, rate limiting, and spam control must remain server-side.

---

## 📚 Documentation Map

Start exploring the architecture and API specifications:

1. **[docs/DOCS-README.md](docs/DOCS-README.md)**: Navigation guide for the documentation tree.
2. **[docs/01-whitepaper/README.md](docs/01-whitepaper/README.md)**: Product goals, transaction flows, and architecture maps.
3. **[docs/01-whitepaper/10-api-contract.md](docs/01-whitepaper/10-api-contract.md)**: Frozen V1 payload formats and schemas.
4. **[docs/specs/openapi.yaml](docs/specs/openapi.yaml)**: Live Swagger/Redocly-linted OpenAPI specification.

### Planning & Deep-Dives
- **[Identity & Auth](docs/01-whitepaper/11-identity-authz.md)**: Cryptographic claims and anti-abuse trust matrices.
- **[Backend Design](docs/server/module-layout.md)**: Rust crate layout, module boundaries, and service architecture.
- **[MCP Tool Catalog](docs/mcp/tool-catalog.md)**: Desktop agent tools and runtime schemas.
- **[Deployment Runbook](docs/deploy.md)**: Production deployment instructions and environment configuration.

---

## 📊 Benchmark Baseline (May 12, 2026)

Under load tests simulating thousands of concurrent agent search operations against a local PostgreSQL database, `oz-market` achieves enterprise-level performance:

| Search Concurrency | Throughput (Public Search) | Throughput (Rotating Auth) | Rate Limit (429) Rate | Avg Response Latency |
|:-------------------|---------------------------:|---------------------------:|----------------------:|---------------------:|
| **100 concurrent** | 57,733 ops/s               | 57,418 ops/s               | 0%                    | < 1.8ms              |
| **200 concurrent** | 57,350 ops/s               | 59,140 ops/s               | 0%                    | < 3.4ms              |
| **500 concurrent** | 51,569 ops/s               | 47,946 ops/s               | 0%                    | < 9.7ms              |

> [!NOTE]
> Diagnostic fixed-rate limits properly throttle abusers at 97-100% rejection rate once they exceed their quota (approx. 1,765 ops/s under stress), ensuring system availability.

---

## 🚀 Future Roadmap & Active Specs

We are implementing a production-grade roadmap to take this monorepo to a multi-million dollar agentic commerce infrastructure. See active specifications:

- **[Spec 0024](docs/specs/_active/0024-distributed-ledger-cache-redis/README.md)**: Redis distributed cache clustering for horizontal scale and pub/sub cache invalidation.
- **[Spec 0025](docs/specs/_active/0025-zero-copy-ffi-serialization/README.md)**: MessagePack binary serialization for high-volume, low-latency Tauri FFI.
- **[Spec 0026](docs/specs/_active/0026-transactional-outbox-pattern/README.md)**: Transactional Outbox pattern for reliable event delivery.
- **[Spec 0027](docs/specs/_active/0027-refresh-token-rotation-jwt-blacklist/README.md)**: JWT refresh token rotation and session-breach detection.

---

## 🛠️ Local Development & Quick Start

Ensure you have Rust, PowerShell Core, and Docker installed.

### 1. Verification Gate (Run Before Committing)
Every commit is validated locally against a multi-stage check script that mirrors the GitHub CI pipeline:
```powershell
.\check.ps1
```

### 2. Standalone Database Setup
Start the local Postgres database with standard configurations:
```powershell
docker compose -p marketplace -f compose.postgres.yml up -d
```

### 3. Run Benchmarks & Integration Tests
To validate ledger and throughput performance:
```powershell
# Run both benchmark and Postgres integration tests
powershell -File backend/server/scripts/run-local-postgres-dev.ps1
```

### 4. Model Context Protocol (MCP) Server Setup
The marketplace MCP server allows AI agents (like Claude Desktop) to connect directly to the marketplace and call tools natively (such as creating listings, searching items, and making offers) on behalf of the user.

To configure the MCP server in Claude Desktop:
1. Open your Claude Desktop configuration file:
   - **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
   - **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
2. Add the package to the `mcpServers` definition:
   ```json
   {
     "mcpServers": {
       "oz-market": {
         "command": "npx",
         "args": [
           "-y",
           "@kardelitaitu/oz-market-mcp"
         ],
         "env": {
           "MARKETPLACE_API_KEY": "your-api-key-here"
         }
       }
     }
   }
   ```
3. Restart Claude Desktop. The `oz-market` tools will now be available for AI agents to call.

---

## 📄 LICENSE

*Permission is granted to use, copy, modify, and distribute this software for non-commercial purposes only. Commercial use is prohibited without explicit written permission from the author.*
