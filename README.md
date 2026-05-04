# Project Marketplace

Single-repo workspace for an AI-agent marketplace with four main delivery areas:

1. `backend/server`
2. `backend/mcp`
3. `mobile/app-android`
4. `mobile/app-ios`

The product goal is:

- a marketplace `server`
- an `MCP server` for desktop agents
- `Android` and `iOS` apps with a user-created free AI agent powered by `openrouter/free`

## Repo Layout

```text
/
  AGENTS.md
  README.md
  docs/
  backend/
    server/
    mcp/
    crates/
      marketplace-core/
      api-contract/
      auth-core/
  mobile/
    app-android/
    app-ios/
```

## Structure Rules

- keep the root level clean
- do not add new product surfaces directly at root
- backend shared logic should live under `backend/crates/`
- HTTP transport belongs in `backend/server`
- MCP transport belongs in `backend/mcp`
- mobile apps should consume the same backend contract, not implement business rules locally

## Docs

Start here:

1. [docs/DOCS-README.md](C:\My Script\project-the-marketplace\docs\DOCS-README.md)
2. [docs/whitepaper/README.md](C:\My Script\project-the-marketplace\docs\whitepaper\README.md)
3. [docs/whitepaper/10-api-contract.md](C:\My Script\project-the-marketplace\docs\whitepaper\10-api-contract.md)
4. [docs/specs/openapi.yaml](C:\My Script\project-the-marketplace\docs\specs\openapi.yaml)

Important planning docs:

- [docs/whitepaper/11-identity-authz.md](C:\My Script\project-the-marketplace\docs\whitepaper\11-identity-authz.md)
- [docs/whitepaper/12-openapi-outline.md](C:\My Script\project-the-marketplace\docs\whitepaper\12-openapi-outline.md)
- [docs/server/module-layout.md](C:\My Script\project-the-marketplace\docs\server\module-layout.md)
- [docs/mcp/tool-catalog.md](C:\My Script\project-the-marketplace\docs\mcp\tool-catalog.md)

## Current Direction

- `Rust` for backend
- `PostgreSQL` for source-of-truth storage
- one frozen AI-facing listing JSON contract
- one shared business rule layer for HTTP, MCP, and mobile clients

## Next Implementation Focus

1. scaffold the Rust backend workspace under `backend/`
2. implement the first API surface from `docs/specs/openapi.yaml`
3. map MCP tools to the same shared backend services
4. define Android and iOS auth/session flows
