# CI Commands - Distributed Ledger Cache Synchronization

Execute these commands to verify implementation of this spec:

```bash
# Compile distributed cache module
cd backend && cargo check --bin oz-market-server

# Run unit tests utilizing a local mock or Redis instance
cargo test --package oz-market-server --lib services::ledger_cache_distributed
```
