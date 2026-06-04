# CI Commands - Write-Ahead Log (WAL) and Async Batch Committer

Execute these commands to verify implementation of this spec:

```bash
# Check compilation
cd backend && cargo check --bin marketplace-server

# Run batch and wal unit tests
cargo test --package marketplace-server --lib services::wal
cargo test --package marketplace-server --lib services::async_committer
```
