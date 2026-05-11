# Decisions

## Decision Log

| Decision | Value | Reason |
| --- | --- | --- |
| Canonical listing read path | `/v1/listings/{listing_id}` | Matches frozen OpenAPI server base `/v1` and path contract |
| Legacy type-specific paths | Out of frozen contract | Keep contract compact and avoid split behavior |
| Service ownership | Shared backend listing service | Prevent business-rule drift across HTTP and MCP |
| Deprecation handling | Explicit headers and timeline | Safer rollout for existing consumers |

## OpenAPI Parity Check (2026-05-11)

- `docs/specs/openapi.yaml` contains `/listings/{listing_id}`.
- `docs/specs/openapi.yaml` does not contain `/product/{listing_id}`, `/service/{listing_id}`, `/property/{listing_id}`.
- Contract uses `servers: /v1`, so canonical external path is `/v1/listings/{listing_id}`.
