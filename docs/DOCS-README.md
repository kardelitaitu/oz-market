# Docs Overview

This `docs` folder is organized by delivery surface and planning stage.

## Current Structure

| Folder | Purpose | Current State |
| --- | --- | --- |
| `whitepaper/` | product, architecture, data, API, auth, and roadmap planning | active |
| `specs/` | implementation-ready specs such as `openapi.yaml` and schema artifacts | active |
| `server/` | server-specific implementation notes, module structure, and backend runbooks | active |
| `mcp/` | MCP server contract, tool definitions, and integration notes | active |
| `app-android/` | Android app architecture, screens, and app-agent integration notes | active |
| `app-ios/` | iOS app architecture, screens, and app-agent integration notes | active |

## Reading Order

If someone is new to the project, read in this order:

1. `whitepaper/README.md`
2. `whitepaper/01-overview.md`
3. `whitepaper/10-api-contract.md`
4. `whitepaper/11-identity-authz.md`
5. `whitepaper/12-openapi-outline.md`

## Product Surfaces

The current end product is:

1. a `server`
2. an `MCP server` for desktop agents
3. `Android` and `iOS` apps with a user-created free AI agent powered by `openrouter/free`

## Folder Intent

### `whitepaper/`

Business and system planning.

Use this folder for:

- product scope
- architecture decisions
- data model decisions
- API contract decisions
- auth and trust model
- roadmap

### `specs/`

Implementation-oriented artifacts.

Use this folder for:

- `openapi.yaml`
- schema definitions
- validation rules
- endpoint checklists

### `server/`

Backend-specific docs.

Use this folder for:

- Rust crate/module plan
- database migration plan
- deployment notes
- performance notes
- observability/runbooks

### `mcp/`

Desktop-agent integration docs.

Use this folder for:

- MCP tool catalog
- tool input/output schemas
- auth flow for MCP clients
- MCP examples and edge cases

### `app-android/`

Android app planning.

Use this folder for:

- app architecture
- screen flow
- mobile auth flow
- app-agent integration behavior

### `app-ios/`

iOS app planning.

Use this folder for:

- app architecture
- screen flow
- mobile auth flow
- app-agent integration behavior

## Best Next Moves

1. refine `docs/specs/openapi.yaml` with negotiation and reservation response schemas
2. add role-permission matrix docs
3. add server migration and deployment notes
4. add detailed Android and iOS navigation/auth docs
