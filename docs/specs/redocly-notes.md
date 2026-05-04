# Redocly Notes

## Goal

Make the chosen Redocly validation direction concrete enough for implementation planning and preserve it as the source policy doc for the future Redocly config.

This is not a full Redocly config yet. It is the documented validator policy the future config should enforce.

## Recommended Role

Use `@redocly/cli` as the main OpenAPI structure validator for:

- OpenAPI `3.1` validity
- unresolved `$ref` detection
- duplicate or weak operation structure detection
- bundled-spec sanity checks

## Recommended First Command

```text
redocly lint docs/specs/openapi.yaml
```

## Project Validation Policy

Redocly should be treated as the structural gate.

It should confirm:

- the spec is valid OpenAPI `3.1`
- all component references resolve
- operation IDs stay unique
- request and response shapes are structurally valid

## Source Of Truth

This note is the current source of truth for project-specific Redocly policy until a real `redocly.yaml` or equivalent config file is added.

## Responsibility Split

| Tool | Main Role |
| --- | --- |
| `yamllint` | YAML syntax and basic YAML quality |
| `@redocly/cli` | OpenAPI structural validation |
| `@stoplight/spectral-cli` | project-specific lint policy |
| `oasdiff` | breaking-change detection |

## Best Next Moves

1. convert this note into a real Redocly config file when CI starts
2. decide whether internal `/internal/v1` validation stays in the same config or a separate one
3. keep Redocly focused on structure and let Spectral carry most custom policy checks
