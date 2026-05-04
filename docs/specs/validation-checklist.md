# Validation Checklist

## Goal

Turn spec-governance policy into an executable checklist for:

- `openapi.yaml`
- future internal API specs
- MCP-facing contract review

## Required Validation Flow

Run these checks for every contract-affecting change:

1. syntax validation
2. OpenAPI structure validation
3. lint and quality review
4. example validation
5. compatibility review
6. whitepaper alignment review

## Validation Commands

Run in this order:

```text
yamllint docs/specs/openapi.yaml
```

```text
npx @redocly/cli lint docs/specs/openapi.yaml
```

```text
npx @stoplight/spectral-cli lint docs/specs/openapi.yaml
```

```text
oasdiff breaking --fail-on-diff docs/specs/baseline/openapi.yaml docs/specs/openapi.yaml
```

`docs/specs/baseline/openapi.yaml` should be created from the last approved spec before the breaking-change check becomes mandatory in CI.

## Checklist

### 1. Syntax Validation

- [ ] `docs/specs/openapi.yaml` parses as valid YAML
- [ ] generated JSON conversion succeeds if used in tooling

### 2. OpenAPI Structure Validation

- [ ] spec validates as OpenAPI `3.1`
- [ ] all `$ref` targets resolve
- [ ] every `operationId` is unique
- [ ] security scheme references resolve

### 3. Contract Quality Validation

- [ ] every write endpoint documents `400`, `401`, and `403`
- [ ] every replay-sensitive write documents `409` and `429`
- [ ] every write endpoint declares `x-audit-required`
- [ ] every write endpoint declares `x-idempotency-required`
- [ ] create and replay-sensitive write request bodies expose `idempotency_key` where V1 requires explicit replay protection
- [ ] every protected operation declares `x-required-scopes`
- [ ] every protected operation declares `x-allowed-roles`
- [ ] public schemas declare `additionalProperties` behavior explicitly where relevant

### 4. Example Validation

- [ ] canonical listing example matches the frozen `schema_version = 1.0` payload
- [ ] search examples match deterministic `sort_by` rules
- [ ] negotiation examples follow allowed state-machine transitions
- [ ] reveal examples return references, not raw phone data

### 5. Compatibility Review

- [ ] no existing field was renamed without explicit versioning decision
- [ ] no existing field type changed unexpectedly
- [ ] no optional field became required without review
- [ ] enum meaning changes are documented
- [ ] breaking changes are reflected in whitepaper and versioning policy

### 6. Whitepaper Alignment Review

- [ ] `docs/whitepaper/10-api-contract.md` still matches the public contract
- [ ] `docs/whitepaper/11-identity-authz.md` still matches auth expectations
- [ ] `docs/whitepaper/14-state-machines.md` still matches stateful responses
- [ ] `docs/whitepaper/20-spec-validation-and-governance.md` still matches the validation process
- [ ] `docs/whitepaper/21-auth-scopes-and-claims.md` still matches scope annotations

## Write Endpoint Policy

| Operation | `x-audit-required` | `x-idempotency-required` |
| --- | --- | --- |
| `POST /v1/listings` | `true` | `true` |
| `POST /v1/negotiations` | `true` | `true` |
| `POST /v1/negotiations/{negotiation_id}/offers` | `true` | `true` |
| `POST /v1/negotiations/{negotiation_id}/request-contact-reveal` | `true` | `true` |
| `POST /v1/contact-reveals/{reveal_id}/approve` | `true` | `false` |

## Recommended Tooling Shape

| Layer | Purpose |
| --- | --- |
| `yamllint` | syntax and basic YAML quality |
| `@redocly/cli` | structural OpenAPI validation |
| `@stoplight/spectral-cli` | quality and consistency linting |
| `oasdiff` | breaking-change review |

## Practical Notes

- `yamllint` is the first fast gate and should fail on basic YAML issues.
- `@redocly/cli` should be the structural gate for OpenAPI conformance.
- `@stoplight/spectral-cli` should carry the project-specific rules.
- `oasdiff` should only run once a baseline spec artifact exists.

## Best Next Moves

1. add a future checklist for `/internal/v1` once that spec exists
2. convert this checklist into automated CI gates before backend code depends heavily on the contract
3. add a future baseline spec artifact for `oasdiff` once the first spec is frozen
