# CI Commands - Dual-Layer Ledger Trait and Synchronous Cache

Execute these commands to verify implementation of this spec:

```bash
# Compile cache service
cd backend && cargo check --bin marketplace-server

# Run service tests
cargo test --package marketplace-server --lib services::ledger_cache
```
