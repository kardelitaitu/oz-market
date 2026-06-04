# Decisions - Agent Circuit-Breaker and Health API

## Architecture Decisions

### 1. Circuit State In-Memory Only
- **Decision**: Keep circuit states in-memory instead of persisting to Postgres.
- **Rationale**: Dynamic health and breaker state is transient and can be rebuilt on startup. Avoiding persistent state simplifies logic and eliminates database write latency overhead during failures.

### 2. Standard 30s Cooldown for Half-Open Transition
- **Decision**: Use a default cooldown window of 30 seconds before trying to probe an agent that tripped the breaker.
- **Rationale**: Gives degraded downstream agent instances sufficient time to restart, scale, or recover before re-routing traffic to them.

### 3. Consecutive-Failure Threshold Replaces Error-Rate Threshold
- **Decision**: Trip the breaker after 5 consecutive failures (or a single response exceeding 2000ms) rather than when the rolling error rate exceeds 20%.
- **Rationale**: At low sample counts, an error-rate threshold can be dominated by a single bad result (e.g. 1 failure in 1 sample = 100% trips the breaker). The consecutive-failure model gives operators a simpler mental model ("the agent has been failing for the last 5 attempts"), is more stable under low traffic, and is straightforward to reason about. The trade-off is that an agent that fails 1 of every 2 requests will never trip the breaker under this model, but that mixed-success pattern is generally tolerable — only sustained failure streaks warrant bypassing an agent.
- **Status**: The `error_threshold_pct` field on `AgentCircuitBreaker` and `CircuitBreakerRegistry` is accepted at construction for API stability but is stored as `_error_threshold_pct` and not consulted. To reactivate the error-rate policy in the future, the `record_result` body in `backend/server/src/services/circuit_breaker.rs` must consult the field.
