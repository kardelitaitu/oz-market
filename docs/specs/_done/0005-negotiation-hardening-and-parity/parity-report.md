# Parity Report

Generated: `2026-05-18`
Spec: `0005-negotiation-hardening-and-parity`
Source Contract: `docs/specs/openapi.yaml`

## Machine-Readable Snapshot

```json
{
  "spec_id": "0005-negotiation-hardening-and-parity",
  "generated_on": "2026-05-18",
  "openapi_server_base": "/v1",
  "target_status_parity": {
    "request_contact_reveal_expected_http_status": 202
  },
  "checks": {
    "reveal_request_ownership_bound_to_negotiation": true,
    "reveal_approval_ownership_bound_to_listing_owner": true,
    "open_negotiation_offer_amount_positive_finite_guard": true,
    "open_negotiation_conflict_compensation_present": true,
    "request_contact_reveal_status_code_parity_202": true,
    "targeted_ownership_conflict_tests_present": true,
    "cargo_check_last_run_passed": true
  },
  "result": "complete"
}
```

## Notes

- All 7 parity checks verified and passing as of 2026-05-18.
- `reveal_request_ownership_bound_to_negotiation`: `request_contact_reveal` in `app.rs` now looks up stored negotiation + listing and calls `authorize_request_contact_reveal` with stored buyer/seller IDs. Verified via `postgres_rejects_outsider_reveal_request` integration test.
- `reveal_approval_ownership_bound_to_listing_owner`: `approve_contact_reveal` in `app.rs` now traverses reveal→negotiation→listing chain and calls `authorize_approve_contact_reveal` with stored listing owner ID. Verified via `postgres_rejects_wrong_seller_reveal_approval` integration test.
- `open_negotiation_offer_amount_positive_finite_guard`: `open_negotiation` now validates offer_amount is positive finite. Verified via `postgres_rejects_open_negotiation_invalid_amount` integration test.
- `open_negotiation_conflict_compensation_present`: upsert-conflict releases reservation + commits idempotency failure. Verified via `postgres_open_negotiation_conflict_compensation` integration test.
- `request_contact_reveal_status_code_parity_202`: confirmed 202 in runtime.rs, actix_handlers.rs, and OpenAPI spec.
- `targeted_ownership_conflict_tests_present`: 5 new Postgres integration tests + existing app-level unit tests cover all ownership/conflict/validation paths.
- `cargo_check_last_run_passed`: confirmed via full `check.ps1` run (all 6 stages pass).
