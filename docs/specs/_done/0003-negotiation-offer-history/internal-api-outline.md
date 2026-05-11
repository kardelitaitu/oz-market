# Internal API Outline

## Shared Service Contract

- `create_negotiation(...) -> NegotiationResponse`
- `submit_offer(...) -> NegotiationResponse` (idempotent write)
- `accept_negotiation(...) -> NegotiationResponse` (reservation-gated)
- `reject_negotiation(...) -> NegotiationResponse`

## Transport Mapping

- HTTP endpoints and MCP tools call the same shared service functions
- transport layers do not implement state-machine branching

## Repository Boundaries

- negotiation repository owns negotiation row + offer history persistence
- reservation repository owns lease verification and final gate checks
