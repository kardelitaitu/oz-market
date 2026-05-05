# Android Polling-First Event Flow

## Goal

Keep Android event awareness on the shared read API first.

## Rules

- poll listing state and negotiation status first
- poll reveal status after approval actions
- treat `409` and replay responses as retry signals
- do not depend on a separate mobile event stream in V1

## Flow

1. user performs a write action
2. app waits briefly
3. app polls the matching read endpoint
4. app updates the UI from the canonical response

## Notes

- keep this aligned with `docs/whitepaper/23-event-delivery.md`
- keep polling behavior identical across Android and iOS
