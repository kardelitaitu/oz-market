# Decisions - Agent Routing and Dispatch Core Layer

## Architecture Decisions

### 1. In-Memory Registry for Agent Instances
- **Decision**: Agent registry metadata will be stored in-memory inside `DashMap` for concurrent read accesses during high-frequency routing queries.
- **Rationale**: Avoids database query latency for agent location mapping on every API request.

### 2. Dispatcher Trait abstraction
- **Decision**: Define routing via a generic `AgentDispatcher` trait.
- **Rationale**: Decouples network/HTTP communication from business logic, allowing mock implementations during local tests.
