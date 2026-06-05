# CI Commands - Agent Metrics Collector

Execute these commands to verify implementation of this spec:

```bash
# Compile metrics service
cd backend && cargo check --bin oz-market-server

# Run metrics unit tests
cargo test --package oz-market-server --lib services::agent_metrics
```
