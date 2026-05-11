# Parity Report

Generated: `2026-05-11`
Spec: `0001-unified-listings-endpoint`
Source Contract: `docs/specs/openapi.yaml`

## Machine-Readable Snapshot

```json
{
  "spec_id": "0001-unified-listings-endpoint",
  "generated_on": "2026-05-11",
  "openapi_server_base": "/v1",
  "checks": {
    "has_listings_by_id_path": true,
    "has_legacy_product_path": false,
    "has_legacy_service_path": false,
    "has_legacy_property_path": false,
    "spec_wording_uses_v1_listing_id": true
  },
  "result": "pass"
}
```

## Notes

- Canonical external read path is `/v1/listings/{listing_id}`.
- Legacy type-specific listing paths are treated as migration behavior, not frozen OpenAPI surface.
