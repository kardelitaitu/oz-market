# CI Commands - Delete Legacy Benchmarks

Execute these commands to verify implementation of this spec:

```bash
# Verify backend compiles cleanly without old binaries
cd backend && cargo check --workspace

# Run testing suite
cargo test --workspace
```
