# Test Strategy

## Goal

Define how the system should be validated before and during implementation.

The test strategy should protect:

- contract stability
- permission correctness
- state transition correctness
- concurrency safety
- abuse controls
- performance claims

## Testing Principles

- test shared business rules once, reuse across HTTP and MCP
- prefer deterministic machine-readable assertions
- treat concurrency and authz as first-class test areas
- do not rely only on happy-path tests

## Test Layers

| Layer | Purpose | Priority |
| --- | --- | --- |
| schema/contract tests | protect payload and endpoint shape | highest |
| unit tests | validate pure logic and state transitions | highest |
| repository tests | validate SQL behavior and constraints | highest |
| integration tests | validate HTTP and MCP surface behavior | high |
| concurrency tests | validate reservation and race safety | high |
| performance tests | validate latency and throughput assumptions | high |
| mobile client contract tests | validate Android/iOS payload compatibility | medium |

## 1. Contract Tests

Protect:

- listing payload shape
- search payload shape
- error response shape
- enum values
- required vs optional field behavior

Required checks:

- invalid enum values fail cleanly
- missing required fields fail cleanly
- optional fields may be omitted
- HTTP and MCP use the same field names
- explicit `idempotency_key` rules are enforced on required write paths

## 2. Authz Tests

Protect:

- role-permission matrix
- ownership rules
- seller vs buyer action separation
- reveal approval restrictions

Required checks:

- seller cannot act as unrelated seller
- buyer cannot approve reveal
- support reviewer cannot mutate business state
- admin overrides remain auditable

## 3. State Machine Tests

Protect:

- listing transitions
- negotiation transitions
- reservation lease transitions
- contact reveal transitions

Required checks:

- valid transitions succeed
- invalid transitions fail with machine-readable errors
- stale version writes fail
- closed/cancelled entities do not reopen incorrectly

## 4. Concurrency Tests

Protect:

- double-sell prevention
- reservation uniqueness
- replay-safe writes
- reveal approval races

Required checks:

- two buyers racing for one listing result in one winner only
- duplicate accept/reveal calls remain idempotent
- lease expiration and release paths are safe under retry
- reservation invariants hold after conflict storms

## 5. Abuse And Quota Tests

Protect:

- seller trust-level quotas
- duplicate listing detection
- rate limits
- replay throttling

Required checks:

- new seller quota is enforced
- trusted seller quota is higher but still enforced
- repeated duplicate listing attempts trigger abuse handling
- repeated reveal requests hit rate or idempotency controls

## 6. Repository And Constraint Tests

Protect:

- one active lease per listing
- version updates
- transactional reservation changes
- search index assumptions
- audit and outbox write integrity

Required checks:

- relational constraints reject illegal states
- version increments only on successful writes
- reservation creation and listing state update happen atomically
- audit and outbox rows are written when required mutations commit

## 7. Audit And Outbox Tests

Protect:

- audit trace completeness
- outbox publication intent
- retry-safe asynchronous delivery boundaries

Required checks:

- stateful writes create expected `audit_events`
- stateful writes create expected `outbox_events` when event delivery is required
- failed delivery retries do not mutate canonical entity state
- outbox consumers can deduplicate by event id

## 8. Integration Tests

### HTTP

Required checks:

- create listing
- get listing
- search listings
- open negotiation
- submit offer
- request reveal
- approve reveal
- create and negotiation-open retries reuse the same result when the same `idempotency_key` is replayed

### MCP

Required checks:

- MCP tool input/output matches HTTP contract
- MCP authz matches HTTP authz
- MCP cannot bypass abuse or reservation rules
- MCP create/open tools preserve required `idempotency_key` behavior

## 9. Mobile Contract Tests

Protect:

- Android and iOS payload compatibility
- app-agent interaction with backend contract
- error handling consistency

Required checks:

- mobile clients send canonical listing payload
- mobile search request maps to canonical search object
- mobile negotiation flows handle conflict and rate-limit errors
- mobile create/open flows generate and reuse `idempotency_key` on safe retries

## 10. Performance Tests

Protect:

- p95/p99 latency targets
- benchmark profile assumptions
- search path behavior under load

Required checks:

- listing-read benchmark
- search-heavy benchmark
- negotiation-burst benchmark
- quota/rate-limit behavior under pressure

## Suggested Test Execution Order

1. contract tests
2. authz tests
3. state machine tests
4. repository tests
5. audit and outbox tests
6. integration tests
7. concurrency tests
8. performance tests
9. mobile contract tests

## Exit Criteria Before Broad Implementation

- contract tests pass
- role-permission matrix is enforced
- invalid transitions fail correctly
- double-sell race test has one winner only
- quota controls are enforced
- initial benchmark results are measured

## Best Next Moves

1. map each whitepaper rule to at least one test category
2. convert this strategy into executable test suites after backend scaffolding
3. keep test names aligned with contract and state-machine terminology
