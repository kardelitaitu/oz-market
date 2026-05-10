# Backend Tests

## Status: In Development

This directory is reserved for backend-wide integration and end-to-end tests.

### Current State

- **No tests implemented yet** — this folder was created in anticipation of future test coverage.
- Unit tests for individual crates live alongside their source code (e.g., `backend/crates/api-contract/src/lib.rs` with `#[cfg(test)]` modules).
- Integration tests for the HTTP server and MCP transport will be added here once the backend scaffolding is complete.

### Planned Structure

```
backend/tests/
├── integration/        # Cross-crate integration tests
│   ├── api_contract_tests.rs
│   └── server_mcp_contract_tests.rs
├── fixtures/           # Test data and mock responses
└── TEST-README.md      # This file
```

### Development Guidelines

1. **Before implementing**: Check `docs/specs/openapi.yaml` for the frozen API contract.
2. **Prefer unit tests** in each crate until integration complexity warrants a shared test suite.
3. **Use the local Postgres container** for database integration tests:
   ```bash
   docker compose -p marketplace -f compose.postgres.yml up -d
   ```

### Related Docs

- [docs/DOCS-README.md](../../../docs/DOCS-README.md)
- [docs/specs/openapi.yaml](../../../docs/specs/openapi.yaml)