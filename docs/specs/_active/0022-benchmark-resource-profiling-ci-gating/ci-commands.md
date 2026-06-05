# CI Commands - Benchmark Resource Profiling and CI Gating

Execute these commands to verify implementation of this spec:

```bash
# Compile server and test profiler
cd backend && cargo build --bin marketplace-server

# Run CI check script containing gating benchmarks
cd .. && powershell -File check.ps1 -SkipBuild -SkipFormat -SkipClippy -SkipTests
```
