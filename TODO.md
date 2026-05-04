# TODO

A checklist of tasks. Use `~~strikethrough~~` only after the task is validated complete through tests or a documented verification step.

## Priority Decision

Do `executable spec validation` first.

## Why This Comes First

| Option | Pros | Cons | Recommendation |
| --- | --- | --- | --- |
| executable spec validation first | locks contract quality early, reduces backend drift, makes scaffold work safer | delays code scaffolding slightly | Best |
| backend scaffold work first | faster visible code progress | high risk of baking unstable contract assumptions into code | Weak |

Short reason:

- the repo already has a strong whitepaper and spec set
- the biggest remaining risk is contract drift, not missing folders
- validating the spec first gives a reliable base for Rust scaffolding

## Current Build Order

### Phase 1: Executable Spec Validation

- [x] ~~final review on `docs/whitepaper`~~
- [x] ~~finalize `docs/specs/openapi.yaml` as the implementation contract~~
- [x] ~~turn `docs/specs/validation-checklist.md` into actual validation commands later~~
- [x] ~~keep `docs/specs/spectral-rules.md` as the lint-policy source~~
- [x] ~~keep `docs/specs/redocly-notes.md` as the validator-policy source~~
- [x] ~~define the first CI command set for:~~
  - [x] ~~`yamllint`~~
  - [x] ~~`@redocly/cli`~~
  - [x] ~~`@stoplight/spectral-cli`~~
  - [x] ~~`oasdiff`~~
- [x] ~~verify contract alignment across:~~
  - [x] ~~HTTP~~
  - [x] ~~MCP~~
  - [x] ~~Android~~
  - [x] ~~iOS~~

### Phase 2: Backend Scaffold Work

- [x] ~~scaffold Rust workspace under `backend/`~~
- [x] ~~create shared crates:~~
  - [x] ~~`backend/crates/marketplace-core`~~
  - [x] ~~`backend/crates/api-contract`~~
  - [x] ~~`backend/crates/auth-core`~~
- [x] ~~scaffold transports:~~
  - [x] ~~`backend/server`~~
  - [x] ~~`backend/mcp`~~
- [x] ~~wire the scaffold to the frozen OpenAPI contract, not ad hoc request shapes~~

### Phase 3: Implementation After Validation

- [ ] relational schema for `listings`
- [ ] relational schema for `negotiations`
- [ ] `reservation_leases`
- [ ] `contact_reveals`
- [ ] `audit_events`
- [ ] `outbox_events`
- [ ] authz enforcement
- [ ] idempotency enforcement
- [ ] indexed search path

## Working Rule

- update this TODO when build order changes
- prefer contract-first implementation
- do not start transport code before validation policy is executable
