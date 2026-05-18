# Implementation Notes

## Current Execution Notes

- Existing app/repository layers already include idempotency and reservation services.
- Existing negotiation tests cover several happy paths and selected guard paths.

## Work Notes for This Spec

- extend tests to outsider/ownership-negative reveal cases
- implement compensation handling for reserve-then-upsert conflict windows
- align OpenAPI response status docs with runtime + actix outputs
- refresh parity report after all fixes
