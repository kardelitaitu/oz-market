# Plan: Negotiation Hardening and Contract Parity

## What Is the Solution

### Step 1: Ownership Binding

1. Resolve reveal request ownership from stored negotiation + listing context, not caller self-declared context.
2. Resolve reveal approval ownership from reveal -> listing ownership mapping.
3. Reject unauthorized callers with explicit forbidden errors.

### Step 2: Open Negotiation Integrity

1. Validate `offer_amount` as positive finite in open-negotiation path.
2. Add compensation path for reserve/upsert conflicts (release reservation + idempotency failure commit).
3. Ensure all post-begin failure branches commit idempotency failure so no pending records remain stuck.

### Step 3: Parity Alignment

1. Align OpenAPI with runtime and actix status behavior for negotiation/contact-reveal endpoints.
2. Keep contract examples and required constraints synchronized with code validations.
3. Update machine-readable parity report after alignment.

### Step 4: Regression Tests

1. Add tests for outsider reveal request rejection.
2. Add tests for wrong-seller reveal approval rejection.
3. Add tests for open-negotiation invalid amount and reserve/upsert conflict compensation.

## Success Metrics

- unauthorized reveal operations are denied deterministically
- reservation/idempotency state remains consistent after open-negotiation conflicts
- contract and transport status codes match for negotiation/reveal routes
- new regression tests fail before fix and pass after fix

## Phased Rollout Plan

1. Implement ownership/context hardening.
2. Implement open-negotiation integrity fixes.
3. Align OpenAPI and parity report.
4. Validate with cargo check + targeted tests.
5. Move to `_done` after acceptance checklist is complete.
