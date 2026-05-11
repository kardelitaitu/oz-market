# Baseline: Negotiation Offer History and Finalization

## What I Find

### Current State

The current contract already exposes:

- `POST /negotiations`
- `GET /negotiations/{id}`
- `POST /negotiations/{id}/offers`
- `POST /negotiations/{id}/request-contact-reveal`
- `POST /contact-reveals/{id}/approve`

The current backend also already has:

- reservation leases
- idempotency on the main write paths
- reservation gating before reveal

### Current Gap

The negotiation repository still stubs `submit_offer`, so append-only offer history and negotiation-row-led updates are not yet implemented there.

### Why This Matters

Richer append-only offer history needs a stable place to live.

If offer data stays only in audit logs, the client cannot render negotiation progress cleanly.

## What I Claim

The next negotiation spec should:

- keep the current state names
- expose richer append-only offer history
- move offer writes to the negotiation row
- keep reservation as the finalization gate
- add explicit `accept` and `reject` actions

## What Is the Proof

1. The current contract already treats negotiation as a first-class workflow.
2. The current runtime already enforces reservation before contact reveal.
3. The current repository layer still lacks the offer write implementation.
4. The client surfaces need richer history if we want usable negotiation UX.
