# Implementation Checklist

## Goal

Turn the whitepaper into an execution order that can be followed without inventing architecture during coding.

## Phase 0: Freeze Inputs

- [ ] confirm `schema_version = 1.0` listing payload stays unchanged
- [ ] confirm `category` and `condition` enums
- [ ] confirm `price.amount` policy for V1
- [ ] confirm initial seller onboarding and trust levels
- [ ] confirm deferred decision owners accept their ownership

## Phase 1: Contract And Auth Foundations

- [ ] finalize `docs/specs/openapi.yaml`
- [ ] define spec lint and compatibility gates from `20-spec-validation-and-governance.md`
- [ ] align MCP tool shapes with the same contract
- [ ] finalize role-permission enforcement from `13-role-permission-matrix.md`
- [ ] finalize token and agent credential model from `11-identity-authz.md`
- [ ] finalize auth scopes and token claims from `21-auth-scopes-and-claims.md`
- [ ] define machine-readable error codes as shared constants

## Phase 2: Data And State Foundations

- [ ] finalize relational schema for `listings`
- [ ] finalize relational schema for `negotiations`
- [ ] add `reservation_leases`
- [ ] add `contact_reveals`
- [ ] add `audit_events`
- [ ] design `outbox_events` from `24-audit-events-and-outbox.md`
- [ ] add optimistic `version` fields to stateful entities
- [ ] encode state transitions from `14-state-machines.md`

## Phase 3: Abuse And Quota Controls

- [ ] implement seller trust levels
- [ ] implement listing quotas by trust level
- [ ] implement per-token and per-IP rate limits
- [ ] implement duplicate listing fingerprinting
- [ ] implement idempotency enforcement on replay-sensitive writes
- [ ] implement reservation conflict handling

## Phase 4: Server Delivery

- [ ] scaffold Rust workspace under `backend/`
- [ ] implement shared domain logic in `backend/crates/marketplace-core`
- [ ] implement shared contract types in `backend/crates/api-contract`
- [ ] implement auth helpers in `backend/crates/auth-core`
- [ ] implement HTTP transport in `backend/server`
- [ ] implement internal admin/support transport boundaries from `22-admin-and-support-surfaces.md`
- [ ] add database migrations
- [ ] add tracing and metrics hooks
- [ ] add audit and outbox write paths for stateful mutations

## Phase 5: MCP Delivery

- [ ] implement MCP transport in `backend/mcp`
- [ ] map all MCP tools to shared service functions
- [ ] verify MCP cannot bypass authz or abuse controls
- [ ] add MCP conflict and retry examples
- [ ] define MCP event consumption model from `23-event-delivery.md`

## Phase 6: Mobile Delivery

- [ ] finalize Android first flows
- [ ] finalize iOS first flows
- [ ] define mobile auth/session lifecycle
- [ ] define mobile agent setup flow using `openrouter/free`
- [ ] ensure mobile clients use canonical backend payloads only
- [ ] define mobile polling or push integration from `23-event-delivery.md`

## Phase 7: Search And Performance

- [ ] implement first PostgreSQL indexes
- [ ] define benchmark profiles from `15-non-functional-requirements.md`
- [ ] run listing-read benchmark
- [ ] run search-heavy benchmark
- [ ] run negotiation-burst benchmark
- [ ] adjust quotas and indexes from measured behavior

## Phase 8: Final Readiness Checks

- [ ] verify all deferred decisions are resolved or consciously postponed
- [ ] verify docs/specs/code use the same contract names
- [ ] verify all write paths are idempotent where required
- [ ] verify reservation and reveal invariants hold under race conditions
- [ ] verify observability signals exist for quotas, conflicts, and errors

## Exit Criteria For First Implementation

- [ ] one canonical contract across HTTP, MCP, Android, and iOS
- [ ] one shared backend business-rule layer
- [ ] explicit authz and abuse controls
- [ ] deterministic state transitions
- [ ] measured baseline performance, not only planned performance
