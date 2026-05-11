# Validation Checklist

## Contract and Docs

- [ ] `docs/specs/openapi.yaml` contains only the unified listing GET path for this flow
- [ ] `docs/01-whitepaper/10-api-contract.md` describes unified listing retrieval behavior
- [ ] endpoint docs and deprecation timeline match the implementation plan

## Transport and Service Consistency

- [ ] HTTP and MCP routes call the same listing service logic
- [ ] no listing business rule is duplicated in transport layers
- [ ] mobile payload expectations still match the contract

## Reliability and Safety

- [ ] `listing_type` is always present in successful listing responses
- [ ] redirect responses include `Deprecation`, `Sunset`, and `Location` headers
- [ ] authz and abuse controls behave the same on old and unified routes
- [ ] idempotent read behavior remains unchanged under repeated calls

