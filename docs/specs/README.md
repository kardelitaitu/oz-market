# Specs Docs

This folder is for implementation-ready specifications.

## Intended Contents

- `openapi.yaml`
- `internal-api-outline.md`
- `validation-checklist.md`
- `spectral-rules.md`
- `redocly-notes.md`
- `ci-commands.md`
- `yamllint.yaml`
- `run-oasdiff-breaking.ps1`
- `.spectral.yaml`
- `redocly.yaml`
- schema component definitions
- request/response validation notes
- generated client/server contract artifacts

## Current Status

Contains the frozen `openapi.yaml` and the active validation policy files. The public API outline still lives in `../whitepaper/12-openapi-outline.md`, and the internal route outline still lives in `./internal-api-outline.md` while the server wires the current `/internal/v1` namespace.

## Next Docs To Add

1. ✅ shared schema component definitions (see `schemas/README.md`)
2. ✅ generated client/server contract notes (see `generated-contract-notes.md`)
3. ✅ internal `/internal/v1` spec (see `internal-api-spec.md`)
4. 🔜 CI workflow file when automation starts (in progress - quota enforcement mostly done)
5. any `/internal/v1` policy docs when the internal surface needs extra validation detail
