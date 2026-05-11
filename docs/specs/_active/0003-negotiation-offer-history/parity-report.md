# Parity Report

Generated: `2026-05-11`
Spec: `0003-negotiation-offer-history`
Source Contract: `docs/specs/openapi.yaml`

## Machine-Readable Snapshot

```json
{
  "spec_id": "0003-negotiation-offer-history",
  "generated_on": "2026-05-11",
  "openapi_server_base": "/v1",
  "checks": {
    "has_negotiations_path": true,
    "has_negotiation_by_id_path": true,
    "has_negotiation_offers_path": true,
    "has_negotiation_accept_path": true,
    "has_negotiation_reject_path": true,
    "has_offer_history_field": true,
    "has_accept_reject_request_schemas": true,
    "routes_match_http_runtime": true
  },
  "result": "pass"
}
```

## Notes

- OpenAPI and runtime now align on `offers`, `accept`, and `reject` negotiation actions.
- Keep this report updated whenever negotiation payload fields or routes change.
