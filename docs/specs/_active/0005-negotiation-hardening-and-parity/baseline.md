# Baseline: Negotiation Hardening and Contract Parity

## What I Find

### Current State

- Negotiation contract supports open/offer/accept/reject and offer history.
- Transport paths exist in runtime and actix for all core negotiation operations.
- Idempotency and reservation layers are integrated into write operations.

### Current Gap

- Reveal request and reveal approval authorization paths are not fully bound to persisted ownership context.
- Open negotiation can reserve before conflicting upsert without explicit compensation guard.
- Open negotiation amount constraints are less explicit than contract constraints.
- OpenAPI and transport behavior still have parity drift in negotiation-connected response statuses.

### Why This Matters

These gaps are high-severity for correctness and security because they affect identity boundaries and state consistency across concurrency paths.

## What I Claim

The next negotiation spec must prioritize ownership hardening and side-effect consistency before additional feature expansion.

## What Is the Proof

1. Authorization context for reveal request/approval is currently caller-derived in app flow.
2. Open negotiation flow performs reserve then upsert; conflict branch needs explicit compensation semantics.
3. Contract constraints and transport behavior are not perfectly synchronized in all reveal-route responses.
