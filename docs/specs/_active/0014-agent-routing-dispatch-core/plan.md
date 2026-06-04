# Plan - Agent Routing and Dispatch Core Layer

## Implementation Steps

1. **Registry Service**:
   - Create `backend/server/src/services/agent_registry.rs`.
   - Implement `AgentRegistry` holding a `DashMap<Uuid, AgentMetadata>`.
   - Provide methods: `register_agent`, `deregister_agent`, and `get_matching_agents(capabilities: &[String])`.

2. **Dispatcher Module**:
   - Create `backend/server/src/services/agent_dispatcher.rs`.
   - Declare `AgentDispatcher` async trait with `dispatch_query` method.
   - Implement `HttpAgentDispatcher` communicating via `reqwest`.
   - Implement `MockAgentDispatcher` for unit/integration tests without network dependencies.
