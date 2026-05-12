# Parity Report

Generated: `2026-05-12`
Spec: `0004-http-benchmark-stability`
Source Contract: `docs/specs/openapi.yaml`

## Machine-Readable Snapshot

```json
{
  "spec_id": "0004-http-benchmark-stability",
  "generated_on": "2026-05-12",
  "openapi_server_base": "/v1",
  "checks": {
    "benchmark_modes_documented": true,
    "benchmark_reports_include_429_and_other_failures": true,
    "runtime_create_listing_status_201": true,
    "runtime_open_negotiation_status_201": true,
    "runtime_request_contact_reveal_status_202": true,
    "artifacts_dated": true,
    "full_checker_last_run_passed": true
  },
  "result": "pass"
}
```

## Notes

- Keep this snapshot updated whenever benchmark command defaults, response-code expectations, or artifact naming rules change.
- Use this file as the audit anchor for benchmark comparability claims.
