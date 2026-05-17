# Quality Rules

1. Ownership checks must use persisted negotiation/reveal context, not fallback self-derived actor IDs.
2. Negotiation open flow must never leave leaked reservation side effects on conflict.
3. Contract status codes and transport status codes must be validated together before merge.
4. Add regression tests for every new guard branch introduced by this spec.
5. Run `cargo check --manifest-path backend/server/Cargo.toml --workspace` before commit.
