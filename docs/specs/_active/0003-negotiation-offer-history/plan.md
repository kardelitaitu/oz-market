# Plan: Negotiation Offer History and Finalization

## What Is the Solution

### Step 1: Contract Shape

1. Define `offer_history` in the negotiation response
2. Keep the current response fields intact
3. Keep current state names unchanged
4. Treat `offer_history` as append-only negotiation events

### Step 2: Service Shape

1. Move offer history handling into the negotiation row
2. Keep reservation as the finalization gate
3. Keep idempotency on offer and finalization writes

### Step 3: Endpoint Shape

1. Keep `POST /negotiations/{id}/offers`
2. Keep explicit `accept` and `reject` endpoints aligned across OpenAPI and runtime
3. Defer `cancel` unless a real product flow needs it

### Step 4: Shared Surfaces

1. Keep HTTP, MCP, and mobile payloads aligned
2. Keep the same authz and reservation checks
3. Keep the same state names across all surfaces

## Success Metrics

- Negotiation history is visible to clients
- Offer writes stay deterministic and replay-safe
- Reservation still prevents double-sell
- No unnecessary state renames

## Phased Rollout Plan

1. Define the response shape and decision log
2. Update service and repository boundaries
3. Add endpoints only after the contract is settled
4. Validate with the current spec governance checks
