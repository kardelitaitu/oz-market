# Spectral Rules Note

## Goal

Document the Spectral lint policy used by the active ruleset.

`docs/specs/.spectral.yaml` is the source of truth.

## Recommended Base

- start from the OpenAPI rules in `@stoplight/spectral-cli`
- add only the project-specific checks required by the whitepaper

## Project-Specific Policy

### Required operation metadata

Protected operations should declare:

- `x-required-scopes`
- `x-allowed-roles`

Write operations should declare:

- `x-audit-required`
- `x-idempotency-required`

### Response policy

Write operations should document:

- `400`
- `401`
- `403`

Replay-sensitive writes should also document:

- `409`
- `429`

### Schema policy

- public schemas should set `additionalProperties` explicitly where relevant
- request and response examples should match the frozen whitepaper contract
- error responses should use the canonical `ErrorResponse`

## Recommended First Custom Checks

| Rule Intent | Why |
| --- | --- |
| require `x-required-scopes` on protected operations | auth model should not stay implicit |
| require `x-allowed-roles` on protected operations | role review should be machine-checkable |
| require `x-audit-required` on writes | audit policy should be explicit |
| require `x-idempotency-required` on writes | retry semantics should be explicit |
| require canonical error response references | keep error handling uniform |

## Suggested CI Command

```text
npx @stoplight/spectral-cli lint docs/specs/openapi.yaml --ruleset docs/specs/.spectral.yaml
```

## Source Of Truth

`docs/specs/.spectral.yaml` is the source of truth for Spectral policy.

## Best Next Moves

1. keep the custom rules small
2. add internal-spec rules later for `/internal/v1`
3. keep lint behavior reliable and easy to understand
