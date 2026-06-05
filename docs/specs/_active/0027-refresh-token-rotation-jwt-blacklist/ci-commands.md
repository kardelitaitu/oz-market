# CI Commands - Refresh Token Rotation and JWT Blacklist

Execute these commands to verify implementation of this spec:

```bash
# Compile auth core crate
cd backend && cargo check --package marketplace-auth-core

# Run token rotation unit tests
cargo test --package marketplace-auth-core --lib token_rotation
```
