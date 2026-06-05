# CI Commands - Write-Ahead Log (WAL) and Async Batch Committer

Execute these commands to verify implementation of this spec:

```bash
# Check compilation
cd backend && cargo check --bin oz-market-server

# Run batch and wal unit tests
cargo test --package oz-market-server --lib services::wal
cargo test --package oz-market-server --lib services::async_committer
```
