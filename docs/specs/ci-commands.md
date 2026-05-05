# CI Commands

## Goal

Define the first concrete command set for executable spec validation.

These are the commands the project should run locally as spec-policy checks for now.
Use them as local or pre-merge checks.

## Command Order

Run in this order:

1. YAML syntax check
2. OpenAPI structural validation
3. Spectral policy lint
4. Breaking-change diff check

## Commands

### 1. YAML Syntax Check

```text
yamllint -c docs/specs/yamllint.yaml docs/specs/openapi.yaml
```

### 2. OpenAPI Structure Check

```text
npx @redocly/cli lint docs/specs/openapi.yaml --config docs/specs/redocly.yaml
```

### 3. Spectral Policy Check

```text
npx @stoplight/spectral-cli lint docs/specs/openapi.yaml --ruleset docs/specs/.spectral.yaml
```

### 4. Breaking-Change Diff Check

```text
pwsh -File docs/specs/run-oasdiff-breaking.ps1 docs/specs/baseline/openapi.yaml docs/specs/openapi.yaml
```

## Baseline Rule

- `docs/specs/baseline/openapi.yaml` should be the last approved spec snapshot
- update the baseline only when a change is intentionally accepted

## Practical Notes

- use `yamllint` first because it is the cheapest failure mode
- use Redocly next because it catches OpenAPI structure issues early
- use Spectral after structural validation because it carries project policy
- use `oasdiff` only when a baseline spec exists
- use the local `oasdiff` wrapper because the upstream CLI exits nonzero on no-change cases

## Best Next Moves

1. keep these as local spec-policy checks while CI stays cargo-check-only
2. keep `docs/specs/baseline/openapi.yaml` as the approved `oasdiff` baseline
3. keep the command order stable so validation behavior stays predictable
