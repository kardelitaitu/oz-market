# Implementation Notes

## Applied Change Pattern

1. replace hardcoded type-prefixed IDs in tests with clean IDs
2. update builders and fixture helpers to emit clean IDs consistently
3. update assertions to use explicit type fields

## Guardrails

- avoid touching runtime handlers, repositories, or migrations
- keep test intent unchanged while adjusting IDs

