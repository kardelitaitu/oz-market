# CI Commands - Agent Circuit-Breaker and Health API

Execute these commands to verify implementation of this spec:

```bash
# Compile circuit breaker and handlers
cd backend && cargo check --bin oz-market-server

# Run unit tests
cargo test --package oz-market-server --lib services::circuit_breaker
```
