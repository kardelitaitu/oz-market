# CI Commands - Refresh Token Rotation and JWT Blacklist

Execute these commands to verify implementation of this spec:

```bash
# Compile auth core crate
cd backend && cargo check --package oz-market-auth-core

# Run token rotation unit tests
cargo test --package oz-market-auth-core --lib token_rotation
```
