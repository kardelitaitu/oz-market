# MCP Documentation

This folder contains the marketplace MCP docs for the desktop-agent surface.

## Current Direction

- Desktop agents use the separate `oz-market-mcp` stdio sidecar.
- Mobile clients use the same backend contract through the app or HTTP adapter, not stdio MCP.
- Authz, quotas, idempotency, and reservation checks stay in the shared server layer.

## Current Status

- `backend/mcp/src/runtime.rs` hosts the stdio `rmcp` sidecar and delegates into `MarketplaceApp`.
- `backend/mcp/src/bin/mcp_tester.rs` is a local harness and now injects explicit launcher claims before spawning the sidecar.
- `backend/mcp/tests/basic_protocol.rs` is the lightweight binary smoke test for initialize + `tools/list`.
- The sidecar expects `MARKETPLACE_MCP_CLAIMS_JSON` from its launcher; `MARKETPLACE_MCP_ALLOW_DEV_CLAIMS=1` is only for local smoke tests.
- The scheduled `mcp-smoke` workflow uses the shared Rust schema bootstrap helper before running the current listing-only MCP smoke path.

## Launcher Contract

- pass `MARKETPLACE_MCP_CLAIMS_JSON` explicitly from the desktop launcher
- pass `MARKETPLACE_MCP_DATABASE_URL` only when the sidecar should use Postgres
- keep `MARKETPLACE_MCP_ALLOW_DEV_CLAIMS=1` for local smoke tests only
- avoid relying on ambient process environment for agent identity or data access

## Public Tool Catalog

See [tool-catalog.md](tool-catalog.md) for the canonical public V1 tools.

## Internal Helpers

Internal admin and support helpers stay on the server-side surface and are not part of the public desktop-agent V1 catalog.

## References

- [tool-catalog.md](tool-catalog.md)
- [../01-whitepaper/07-mcp-server.md](../01-whitepaper/07-mcp-server.md)
- [../01-whitepaper/10-api-contract.md](../01-whitepaper/10-api-contract.md)
- [../specs/openapi.yaml](../specs/openapi.yaml)
