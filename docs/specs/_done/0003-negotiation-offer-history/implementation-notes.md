# Implementation Notes

## Service-first Rule

- keep negotiation state transitions in shared backend service logic
- HTTP and MCP transports only map request/response and auth context

## Data Integrity Rules

- append exactly one history entry per accepted state-changing write
- enforce idempotency keys on replay-sensitive writes
- preserve monotonic history ordering by `created_at`

## Rollout Notes

1. finalize response schema and endpoint contract first
2. implement repository/service changes second
3. implement transport bindings last after contract lock
