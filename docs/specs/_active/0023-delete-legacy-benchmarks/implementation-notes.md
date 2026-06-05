# Implementation Notes - Delete Legacy Benchmarks

## Codebase Audit Guidelines

When deleting files and references:

1. **Verify Binary Declarations**:
   Ensure `backend/server/Cargo.toml` has all associated entries deleted. For example, remove:
   ```toml
   [[bin]]
   name = "phase5_bench"
   path = "src/bin/phase5_bench.rs"
   ```

2. **Verify Benchmark Declarations**:
   Remove Criterion references:
   ```toml
   [[bench]]
   name = "search_bench"
   harness = false
   ```

3. **Verify Shell Scripts**:
   Delete `.ps1` files from filesystem.
