# Validation Checklist - Agent Circuit-Breaker and Health API

This checklist is used to confirm the completion of Spec 0017:

- [ ] `AgentCircuitBreaker` and its state machine are implemented in `backend/server/src/services/circuit_breaker.rs`.
- [ ] Router intercepts dispatches and skips/bypasses agents whose state is `Open`.
- [ ] Endpoints `GET /v1/health/agents` and `GET /v1/health/agents/{id}` are functional.
- [ ] Circuit breaker successfully enters `Half-Open` state after cooldown and resolves back to `Closed` or `Open` based on probe result.
- [ ] OpenAPI paths and schemas in `docs/specs/openapi.yaml` are compile-checked with Redocly.
