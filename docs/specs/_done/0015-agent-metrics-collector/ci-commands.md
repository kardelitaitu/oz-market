# CI Commands - Agent Metrics Collector

Execute these commands to verify implementation of this spec:

```bash
# Compile metrics service
cd backend && cargo check --bin marketplace-server

# Run metrics unit tests
cargo test --package marketplace-server --lib services::agent_metrics
```
