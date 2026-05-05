# Android Canonical Payload

## Goal

Keep Android on the same canonical backend payload as HTTP and MCP.

## Rules

- use the frozen API contract as the source of truth
- send the same listing payload shape as `docs/specs/openapi.yaml`
- keep search requests aligned with the canonical search object
- reuse the same `idempotency_key` for safe retries
- do not invent Android-only request or response fields

## Shared Shapes

- listing create envelope
- listing summary response
- negotiation open envelope
- contact reveal request and approval flow

## Notes

- keep this aligned with `docs/whitepaper/10-api-contract.md`
- keep this aligned with `docs/whitepaper/19-test-strategy.md`
