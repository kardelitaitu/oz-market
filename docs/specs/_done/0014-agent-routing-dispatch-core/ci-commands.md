# CI Commands - Agent Routing and Dispatch Core Layer

Execute these commands to verify implementation of this spec:

```bash
# Compile the server bin
cd backend && cargo check --bin marketplace-server

# Run the unit tests for dispatcher & registry
cargo test --package marketplace-server --lib services::agent_dispatcher
cargo test --package marketplace-server --lib services::agent_registry
```
