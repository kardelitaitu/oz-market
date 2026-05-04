# Spectral Rules Note

## Goal

Make the chosen Spectral lint direction concrete enough for implementation planning and preserve it as the source policy doc for the future Spectral config.

This is not a full ruleset file yet. It is the documented lint policy the future ruleset should enforce.

## Recommended Base

- start from the OpenAPI rules in `@stoplight/spectral-cli`
- add small project-specific checks only where the whitepaper requires them

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
spectral lint docs/specs/openapi.yaml
```

## Source Of Truth

This note is the current source of truth for project-specific Spectral policy until a real `.spectral.yaml` or equivalent config file is added.

## Best Next Moves

1. convert this note into a real Spectral rules file when repo automation starts
2. add internal-spec rules later for `/internal/v1`
3. keep custom rules small so lint stays reliable and easy to understand
