# CI Commands - Benchmark Component Drivers

Execute these commands to verify implementation of this spec:

```bash
# Compile drivers and server
cd backend && cargo check --bin oz-market-server

# Run unit and driver mock tests
cargo test --package oz-market-server --lib services::ledger_cache
```
