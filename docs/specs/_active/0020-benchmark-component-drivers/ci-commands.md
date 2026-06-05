# CI Commands - Benchmark Component Drivers

Execute these commands to verify implementation of this spec:

```bash
# Compile drivers and server
cd backend && cargo check --bin marketplace-server

# Run unit and driver mock tests
cargo test --package marketplace-server --lib services::ledger_cache
```
