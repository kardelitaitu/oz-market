# Repository Instructions

## Goal

Keep this codebase:

- reliable
- scalable
- easy to use
- compact at the root level

## Workspace Layout

This repository uses a grouped workspace layout:

```text
backend/
  server/
  mcp/
  crates/
    api-contract/
    auth-core/
mobile/
  app-android/
  app-ios/
docs/
```

## Root Rules

- keep the root level clean
- root should contain only high-signal project files such as `AGENTS.md`, `README.md`, and top-level folders
- do not add `server`, `mcp`, `app-android`, or `app-ios` directly at root
- new backend shared code should prefer `backend/crates/`

## Ownership Rules

- `backend/server`: HTTP API transport and server runtime
- `backend/mcp`: MCP transport and desktop-agent integration
- `backend/crates/api-contract`: shared typed contracts derived from the frozen API contract
- `backend/crates/auth-core`: auth, permission, and identity helpers
- `mobile/app-android`: (deprecated — see `mobile/marketplace/`)
- `mobile/app-ios`: (deprecated — see `mobile/marketplace/`)
- `mobile/marketplace`: Tauri v2 + Svelte 5 mobile app (Android + iOS)

## Architecture Rules

- HTTP and MCP must call the same backend service logic
- mobile clients must use the same backend contract
- do not duplicate business rules across transports
- keep authz and abuse controls server-side
- prefer explicit schemas and deterministic JSON over implicit behavior

## Docs Rules

Before major changes, check:

1. `docs/DOCS-README.md`
2. `docs/whitepaper/README.md`
3. `docs/whitepaper/10-api-contract.md`
4. `docs/specs/openapi.yaml`

For backend structure, check:

- `docs/server/module-layout.md`

For MCP behavior, check:

- `docs/mcp/tool-catalog.md`

## Context Gathering Rule

- before making any code change, spawn multiple file-picker and code-searcher agents in parallel to gather the latest context
- prefer 2-5 parallel agents to explore different parts of the codebase simultaneously
- only edit files after gathering sufficient context

## Implementation Bias

- prefer `Rust` for backend code
- prefer explicit SQL and strong typed contracts
- keep the listing/search payload aligned with the frozen V1 contract
- avoid reviving removed core crates unless they solve a real long-term dependency boundary
- avoid introducing new root-level structure unless there is a strong repo-wide reason

## Change Logging Rule

- **always** write to `JOURNAL.md` after making any code changes
- summarize the changes briefly in the response
- journal entries must record what changed and why, not just file names
- use a `## YYYY-MM-DD HH:MM` heading matching the existing journal format

## Follow-up Rule

- after completing a task, always suggest 4-5 concrete next moves using `suggest_followups`
- suggestions should be actionable and relevant (e.g., "Add tests", "Refactor X", "Continue to next step")
- prefer diverse suggestions covering different directions (testing, polish, next feature, cleanup)

## Workflow Rules

- **NEVER** `git push` without being specifically asked to do it
- always run `check.ps1` before any `git push` (script to be developed)
- when in doubt, commit locally but do not push
- if user asks to "commit" without specifying "push", only commit locally
