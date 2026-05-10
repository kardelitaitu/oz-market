# Backend Test Development Plan

## Goal
Create a comprehensive test suite for the core backend functionality, covering both **unit** and **integration** tests. Tests will live under `backend/tests/` as a dedicated umbrella, while crate‑specific unit tests remain next to the source code.

## Structure
```
backend/tests/
├── integration/          # Cross‑crate integration / end‑to‑end tests
│   ├── api_contract.rs   # Validate the frozen API contract (serialization, schema)
│   ├── auth_core.rs      # Auth flow, permission checks, token handling
│   └── server_mcp.rs     # HTTP server + MCP transport interaction
├── fixtures/             # Static JSON/YAML fixtures, mock DB data
│   ├── api_contract.json
│   └── auth_user.yaml
├── utils.rs              # Helper functions (setup DB, start test server, etc.)
└── TEST-README.md        # Explains status and guidelines (already present)
```

## Test Types
| Type | Location | Purpose |
|------|----------|---------|
| **Unit** | Inside each crate (`src/*.rs` with `#[cfg(test)]`) | Test pure Rust logic, data structures, and helper functions. |
| **Integration** | `backend/tests/integration/` | Spin up the full backend (Actix server, MCP layer, DB) and exercise public interfaces. |
| **Contract** | `backend/tests/integration/api_contract.rs` | Ensure the generated Rust types match the OpenAPI contract (`docs/specs/openapi.yaml`). |
| **Auth** | `backend/tests/integration/auth_core.rs` | Verify JWT creation/validation, permission enforcement, and abuse‑control checks. |
| **E2E** | `backend/tests/integration/server_mcp.rs` | End‑to‑end request/response flow across HTTP and MCP transports. |

## Development Steps
1. **Create Directory Skeleton** (already done for `tests/`).
2. **Add Helper Utilities** (`utils.rs`):
   - Start a temporary PostgreSQL container (or use the existing local dev DB).
   - Boot the Actix server in a background task.
   - Provide a `client()` returning an `reqwest::Client` pre‑configured for the test server.
3. **Write Fixtures**:
   - `fixtures/api_contract.json` – example payload matching the V1 contract.
   - `fixtures/auth_user.yaml` – mock user with roles/permissions.
4. **Implement Core Integration Tests**:
   - **api_contract.rs**: deserialize `api_contract.json` into generated Rust structs (`marketplace_api_contract`) and assert round‑trip integrity.
   - **auth_core.rs**: generate a JWT for a mock user, then verify claims, expiry, and permission checks using `marketplace_auth_core`.
   - **server_mcp.rs**: send a request through the HTTP server (using `reqwest`) and through the MCP client, asserting identical responses.
5. **CI Integration**:
   - Add a step to the GitHub Actions workflow (or local CI) that runs `cargo test --workspace --all-features`.
   - Ensure the Postgres container is started before the test step.
6. **Documentation**:
   - Update `docs/DOCS-README.md` with a *Testing* section linking to this plan.
   - Keep `TEST-README.md` up‑to‑date with the current coverage status.

## Timeline (tentative)
| Week | Milestone |
|------|-----------|
| 1 | Scaffold directories, write `utils.rs` and fixture files. |
| 2 | Implement `api_contract.rs` and achieve 100 % deserialization coverage. |
| 3 | Implement `auth_core.rs` (JWT round‑trip, permission checks). |
| 4 | Implement `server_mcp.rs` (end‑to‑end request flow). |
| 5 | Integrate tests into CI, add flake‑avoidance retries, document results. |

## Ownership
- **Primary**: Backend team lead (e.g., *Alice*). 
- **Reviews**: Rust‑auditor skill, CI‑pipeline maintainers.

---
*This plan is a living document; updates should be reflected in the `backend/tests/TEST-README.md` and in the repository’s main documentation.*
