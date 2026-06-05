# CI Commands - Transactional Outbox Pattern

Execute these commands to verify implementation of this spec:

```bash
# Compile outbox publisher service
cd backend && cargo check --bin oz-market-server

# Run outbox publisher unit tests
cargo test --package oz-market-server --lib services::outbox_publisher
```
