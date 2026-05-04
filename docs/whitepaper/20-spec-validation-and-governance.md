# Spec Validation And Governance

## Goal

Define how the API and MCP contract stays:

- machine-valid
- backward-aware
- reviewable
- consistent across HTTP, MCP, and mobile

The whitepaper should not only define the contract. It should also define how contract drift is prevented.

## Core Rule

`docs/specs/openapi.yaml` is a controlled artifact, not a casual note.

Every contract change should pass:

- machine validation
- compatibility review
- example review
- whitepaper alignment review

## Validation Layers

| Layer | Purpose | Recommendation |
| --- | --- | --- |
| YAML parse validation | catch syntax failures | required |
| OpenAPI schema validation | catch invalid spec structure | required |
| lint rules | catch weak descriptions, missing responses, inconsistent operation shapes | required |
| example validation | catch broken request/response examples | required |
| contract diff review | catch breaking surface changes | required |

## Required Validation Gates

### Gate 1: Syntax

- spec file must parse as valid YAML
- generated JSON representation must succeed

### Gate 2: OpenAPI Structure

- the spec must validate as OpenAPI `3.1`
- all `$ref` targets must resolve
- all operation IDs must stay unique

### Gate 3: Contract Quality

- every write endpoint must document `400`, `401`, `403`
- every replay-sensitive write must document `409` and `429`
- error responses must use one canonical `ErrorResponse`
- public schemas must declare `additionalProperties` behavior explicitly

### Gate 4: Example Quality

- canonical listing example must match the frozen `schema_version = 1.0` payload
- search examples must match deterministic `sort_by` rules
- negotiation examples must follow state-machine rules
- reveal examples must return only controlled references, never raw phone data

## Compatibility Policy

### Non-Breaking Changes

- adding optional fields
- adding new documented examples
- adding new endpoints
- adding new error codes that old clients can safely ignore

### Breaking Changes

- renaming existing fields
- changing existing field types
- changing enum meaning
- making optional fields required
- changing auth behavior without new documentation

## Versioning Rule

| Option | Pros | Cons |
| --- | --- | --- |
| single `v1` with disciplined additive changes | compact, simple, easy for agents | requires strong review discipline |
| frequent version forks | easier to isolate major changes | creates integration drift faster |

Recommendation:

- keep one `v1` surface for now
- prefer additive changes only
- use a new `schema_version` only when payload meaning changes
- use a new API version only when transport-level behavior truly breaks

## Change Control Rule

Every contract-affecting change should update all of:

- `docs/specs/openapi.yaml`
- relevant `docs/whitepaper/*.md` source docs
- MCP tool examples if affected
- mobile examples if affected

## Required Review Checklist

- does this change break weaker agents?
- does this change break MCP tool input or output shape?
- does this change break mobile request or response parsing?
- does this change preserve deterministic search and state semantics?
- does this change keep error codes machine-readable?

## Recommended Tooling Shape

Recommended first CI tools:

| Layer | Selected Tool | Purpose |
| --- | --- | --- |
| YAML parse check | `yamllint` | syntax and basic YAML quality |
| OpenAPI validation | `@redocly/cli` | OpenAPI `3.1` validation and bundle checks |
| OpenAPI lint rules | `@stoplight/spectral-cli` | style, consistency, and custom rules |
| contract diff check | `oasdiff` | breaking-change detection against prior spec |

Recommended first CI gates:

- `yamllint docs/specs/openapi.yaml`
- `redocly lint docs/specs/openapi.yaml`
- `spectral lint docs/specs/openapi.yaml`
- `oasdiff breaking --fail-on-diff <base-spec> docs/specs/openapi.yaml`

## Failure Policy

The spec should block merge when:

- YAML does not parse
- OpenAPI validation fails
- canonical examples are invalid
- undocumented breaking changes are detected

## Best Next Moves

1. freeze the exact CI command set in repo automation later
2. define the first baseline spec artifact for `oasdiff` comparison
3. mirror the same validation idea for MCP tool manifests later
