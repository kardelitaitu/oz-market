# Quality Rules

- clean IDs in tests must be type-agnostic (no prefix-coupled semantics)
- listing type checks must use `listing_type`, not ID parsing
- changes must stay scoped to test files and test support only
- production endpoint behavior and schema stay unchanged
- test fixtures should stay deterministic and readable for future maintenance

