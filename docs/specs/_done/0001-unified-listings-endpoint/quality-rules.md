# Quality Rules

- keep one backend listing service as the source of business truth
- HTTP handler and MCP transport must call the same service entrypoint
- keep response schema aligned with `docs/specs/openapi.yaml` and `docs/01-whitepaper/10-api-contract.md`
- keep response JSON deterministic (`listing_type` required and stable field names)
- keep authz and abuse controls server-side; transport must not bypass checks
- deprecation behavior must be explicit (`Deprecation`, `Sunset`, and `Location` headers)

