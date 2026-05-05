# Redocly Notes

## Goal

Document the Redocly validation policy used by the active config.

`docs/specs/redocly.yaml` is the source of truth.

## Recommended Role

Use `@redocly/cli` as the structural validator for:

- OpenAPI `3.1` validity
- unresolved `$ref` detection
- duplicate or weak operation structure detection
- bundled-spec sanity checks

## Recommended First Command

```text
npx @redocly/cli lint docs/specs/openapi.yaml --config docs/specs/redocly.yaml
```

## Project Validation Policy

Redocly is the structural gate. It should confirm:

- the spec is valid OpenAPI `3.1`
- all component references resolve
- operation IDs stay unique
- request and response shapes are structurally valid

## Source Of Truth

`docs/specs/redocly.yaml` is the source of truth for Redocly policy.

## Responsibility Split

| Tool | Main Role |
| --- | --- |
| `yamllint` | YAML syntax and basic YAML quality |
| `@redocly/cli` | OpenAPI structural validation |
| `@stoplight/spectral-cli` | project-specific lint policy |
| `oasdiff` | breaking-change detection |

## Best Next Moves

1. keep Redocly focused on structure
2. decide whether internal `/internal/v1` validation belongs in the same config or a separate one
3. let Spectral carry the custom policy checks
