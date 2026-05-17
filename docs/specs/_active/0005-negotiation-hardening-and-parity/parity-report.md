# Parity Report

Generated: `2026-05-12`
Spec: `0005-negotiation-hardening-and-parity`
Source Contract: `docs/specs/openapi.yaml`

## Machine-Readable Snapshot

```json
{
  "spec_id": "0005-negotiation-hardening-and-parity",
  "generated_on": "2026-05-12",
  "openapi_server_base": "/v1",
  "target_status_parity": {
    "request_contact_reveal_expected_http_status": 202
  },
  "checks": {
    "reveal_request_ownership_bound_to_negotiation": false,
    "reveal_approval_ownership_bound_to_listing_owner": false,
    "open_negotiation_offer_amount_positive_finite_guard": false,
    "open_negotiation_conflict_compensation_present": false,
    "request_contact_reveal_status_code_parity_202": false,
    "targeted_ownership_conflict_tests_present": false,
    "cargo_check_last_run_passed": true
  },
  "result": "in_progress"
}
```

## Notes

- This spec starts from a hardening baseline where several parity and ownership checks are intentionally marked `false`.
- Flip each check to `true` only after implementation and tests confirm the behavior.
- Explicit parity target: `POST /v1/negotiations/{negotiation_id}/request-contact-reveal` must return `202 Accepted` in runtime, actix, and OpenAPI.
