# Validation Checklist

## Goal

Validate the negotiation spec change before implementation.

## Checklist

- [ ] `offer_history` is defined in the response shape
- [ ] `offer_history` is append-only and ordered by `created_at`
- [ ] `submit_offer` is negotiation-row-led
- [ ] `accept` and `reject` are explicit actions
- [ ] `cancel` stays deferred unless needed
- [ ] current negotiation state names remain unchanged
- [ ] reservation stays the final gate for reveal and finalization
- [ ] HTTP, MCP, and mobile keep the same payload shapes
- [ ] idempotency remains required on replay-sensitive writes

## Review Notes

- compare the new negotiation spec against `docs/specs/openapi.yaml`
- compare the new negotiation spec against `docs/01-whitepaper/09-concurrency-and-reservations.md`
- compare the new negotiation spec against `docs/01-whitepaper/14-state-machines.md`
- compare the new negotiation spec against `docs/01-whitepaper/21-auth-scopes-and-claims.md`
