# Validation Checklist

- [x] Active spec metadata fields are defined (`id`, `status`, `owner`, `implementer`, `acceptance`).
- [x] Plan and baseline are written with explicit ownership and consistency goals.
- [x] Machine-readable parity report snapshot exists.
- [ ] Ownership-negative tests for reveal request and reveal approval pass.
- [ ] Open-negotiation conflict compensation test passes.
- [ ] Post-begin failure branches mark idempotency as failed (no stuck pending records).
- [ ] Request-contact-reveal parity is fixed at `202 Accepted` in runtime, actix, and OpenAPI.
- [ ] OpenAPI and transport parity verified for negotiation/reveal routes.
- [ ] Cargo check pass recorded after final updates.
- [ ] Spec moved to `_done` after acceptance criteria are confirmed.
