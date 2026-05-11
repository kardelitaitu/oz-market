# Implementation Notes

## Scope Boundaries

- no database schema change for this spec
- no search index change for this spec
- no mutation endpoint behavior change

## Implementation Sequence

1. Wire unified `get_listing` path to existing shared listing service logic.
2. Keep legacy endpoints as wrappers that emit deprecation headers.
3. Switch wrappers to 301 redirect after the deprecation window.
4. Remove wrappers only after client migration checks pass.

## Verification Notes

- verify parity across HTTP and MCP for response payload and error mapping
- verify legacy endpoint observability before final removal

