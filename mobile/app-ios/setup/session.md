# iOS Session Setup

## Goal

Define the iOS runtime session lifecycle for app-scoped agent actions.

## Flow

1. user signs in
2. app obtains a short-lived backend token
3. app caches the token only for the active session window
4. app refreshes or re-authenticates when the token expires

## Rules

- session is short-lived
- session is separate from seller identity
- session must not bypass backend permission checks
- retry-safe writes keep the same `idempotency_key`
