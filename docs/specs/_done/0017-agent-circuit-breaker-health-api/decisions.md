# Decisions - Agent Circuit-Breaker and Health API

## Architecture Decisions

### 1. Circuit State In-Memory Only
- **Decision**: Keep circuit states in-memory instead of persisting to Postgres.
- **Rationale**: Dynamic health and breaker state is transient and can be rebuilt on startup. Avoiding persistent state simplifies logic and eliminates database write latency overhead during failures.

### 2. Standard 30s Cooldown for Half-Open Transition
- **Decision**: Use a default cooldown window of 30 seconds before trying to probe an agent that tripped the breaker.
- **Rationale**: Gives degraded downstream agent instances sufficient time to restart, scale, or recover before re-routing traffic to them.
