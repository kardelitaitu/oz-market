# CI Commands - Benchmark Distributed Protocol

Execute these commands to verify implementation of this spec:

```bash
# Compile gRPC protobuf interfaces
cd backend && cargo build --bin oz-market-server

# Run clustering connection integration tests
cargo test --package oz-market-server --test distributed_benchmarks
```
