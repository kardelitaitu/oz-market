# Android `openrouter/free` Setup

## Goal

Define the first Android agent setup path using `openrouter/free`.

## Flow

1. user enables the app agent
2. app selects `openrouter/free` as the default V1 provider
3. app stores provider choice in app settings
4. app keeps provider setup separate from seller identity

## Rules

- use the provider only through app-scoped setup
- do not expose provider choice as a backend trust anchor
- keep the setup path aligned with backend permission checks
- treat provider fallback as unresolved until V1 policy is finalized
