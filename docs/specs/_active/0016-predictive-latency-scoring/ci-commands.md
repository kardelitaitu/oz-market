# CI Commands - Predictive Latency Scoring

Execute these commands to verify implementation of this spec:

```bash
# Compile scoring service
cd backend && cargo check --bin marketplace-server

# Run scoring mathematical unit tests
cargo test --package marketplace-server --lib services::latency_scorer
```
