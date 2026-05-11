# Validation Checklist

## Scope Safety

- [ ] only test/test-support files changed
- [ ] no production behavior, OpenAPI schema, or migration files changed

## Data Consistency

- [ ] no `product-`, `service-`, or `property-` ID patterns remain in tests
- [ ] test builders generate clean IDs by default
- [ ] assertions verify `listing_type` instead of ID prefix

## Reliability

- [ ] full relevant backend test suite passes after cleanup
- [ ] fixture names remain descriptive enough for failure debugging

