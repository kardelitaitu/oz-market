# CI Commands - Cache Invalidation and Admin Interventions

Execute these commands to verify implementation of this spec:

```bash
# Check compilation
cd backend && cargo check

# Run http handler tests
cargo test --package oz-market-server --lib http::handlers::tests::admin_credits
```
# Specs Documentation
