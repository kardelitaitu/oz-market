# CI Commands - Agent Circuit-Breaker and Health API

Execute these commands to verify implementation of this spec:

```bash
# Compile circuit breaker and handlers
cd backend && cargo check --bin marketplace-server

# Run unit tests
cargo test --package marketplace-server --lib services::circuit_breaker
```
