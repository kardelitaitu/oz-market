# Android App Scaffold

This folder is the Android client scaffold.

## Contract Hook

- frozen API contract: `../../docs/specs/openapi.yaml`
- contract notes: `../../docs/app-android/first-user-flow.md`

## Scaffold Layout

- `contract/identity.md` for seller and agent identity mapping
- `contract/payload.md` for canonical request and response shapes
- `setup/session.md` for short-lived token lifecycle
- `setup/openrouter-free.md` for the first agent provider setup
- `ui/polling.md` for polling-first event handling
- `ui/` for first-flow shells

## Current Scope

- keep the client aligned with the frozen HTTP contract
- avoid introducing alternate request or response shapes
- add native UI and networking code later
