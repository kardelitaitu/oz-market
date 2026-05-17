# Decisions: Negotiation Hardening and Contract Parity

## Decision 1: Ownership Source of Truth

Authorization must resolve ownership from persisted negotiation/reveal-linked data, not from caller-provided contextual fallback fields.

## Decision 2: Compensating Conflict Path

When open negotiation fails after reservation side effects, compensation is mandatory to avoid leaked reservation/idempotency state.

## Decision 3: Contract-Transport Lockstep

OpenAPI response/status behavior is a first-class contract and must match both runtime and actix implementations.

## Alternatives

| Option | Pros | Cons |
|---|---|---|
| App-layer hardening only | faster initial implementation | easier to regress if repo layer stays permissive |
| Repository-layer hardening only | strong data-bound invariant | weaker API-layer clarity, harder error mapping |
| App + repository hardening (chosen) | defense-in-depth, explicit and durable | more changes and tests required |
