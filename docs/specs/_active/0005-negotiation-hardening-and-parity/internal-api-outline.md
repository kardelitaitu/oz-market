# Internal API Outline

## Negotiation Routes

- `POST /v1/negotiations`
- `GET /v1/negotiations/{negotiation_id}`
- `POST /v1/negotiations/{negotiation_id}/offers`
- `POST /v1/negotiations/{negotiation_id}/accept`
- `POST /v1/negotiations/{negotiation_id}/reject`

## Contact Reveal Routes

- `POST /v1/negotiations/{negotiation_id}/request-contact-reveal`
- `POST /v1/contact-reveals/{reveal_id}/approve`

## Validation Focus

- ownership enforcement path for reveal request and approval
- amount validation for open and submit offer paths
- transport/status-code parity for reveal request and approval
- idempotency + reservation consistency under conflict
