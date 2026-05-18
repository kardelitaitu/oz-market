# Validation Checklist

- [x] Active spec metadata fields are defined (`id`, `status`, `owner`, `implementer`, `acceptance`).
- [x] Plan and baseline are written with explicit ownership and consistency goals.
- [x] Machine-readable parity report snapshot exists.
- [x] Ownership-negative tests for reveal request and reveal approval pass.
- [x] Open-negotiation conflict compensation test passes.
- [x] Post-begin failure branches mark idempotency as failed (no stuck pending records).
- [x] Request-contact-reveal parity is fixed at `202 Accepted` in runtime, actix, and OpenAPI.
- [x] OpenAPI and transport parity verified for negotiation/reveal routes.
- [x] Cargo check pass recorded after final updates.
- [ ] Spec moved to `_done` after acceptance criteria are confirmed.
