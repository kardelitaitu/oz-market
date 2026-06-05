# CI Commands - Benchmark CLI and Standalone Engine

Execute these commands to verify implementation of this spec:

```bash
# Compile the benchmark binary target
cd backend && cargo check --bin bench-suite

# Run standalone benchmark suite tests
cargo test --package marketplace-server --bin bench-suite
```
