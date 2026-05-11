# Decisions

## Decision Log

| Decision | Value | Reason |
| --- | --- | --- |
| ID format in tests | Clean IDs without type prefixes | Decouple identifiers from type semantics |
| Type assertions | Use `listing_type` field | Keep tests aligned with explicit contract behavior |
| Scope | Test and test-support files only | Avoid production-risk changes |

## Completion Notes

- This spec remains `completed` because scope is test-only and governance docs are now explicit.
- No OpenAPI or runtime endpoint behavior change is part of this spec.
