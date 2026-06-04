# Plan - Agent Circuit-Breaker and Health API

## Implementation Steps

1. **State Machine Definition**:
   - Create `backend/server/src/services/circuit_breaker.rs`.
   - Define `CircuitState` enum (`Closed`, `Open`, `HalfOpen`).
   - Define `AgentCircuitBreaker` struct storing state, thresholds, cooldowns, and timestamps.

2. **Integration with Dispatch**:
   - Update query routing logic. Before dispatching to an agent, check its circuit state. If `Open`, skip or return a fallback error.
   - If `HalfOpen`, allow a single probe request. If it succeeds within acceptable thresholds, close the circuit; otherwise trip back to `Open`.

3. **HTTP API Controllers**:
   - Implement Actix web endpoints:
     - `GET /v1/health/agents`
     - `GET /v1/health/agents/{id}`
   - Expose these paths in the OpenAPI specification (`docs/specs/openapi.yaml`).
