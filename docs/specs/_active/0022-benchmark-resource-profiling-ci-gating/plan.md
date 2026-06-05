# Plan - Benchmark Resource Profiling and CI Gating

## Implementation Steps

1. **Add sysinfo Dependency**:
   - Add `sysinfo` dependency to `backend/server/Cargo.toml`.

2. **Telemetry Sampler Thread**:
   - Code background sampling using `sysinfo::System`.
   - Record average CPU percentage, peak memory bytes, and disk write throughput.

3. **Performance Gating Logic**:
   - Implement configuration parser for validation limits.
   - Code threshold checker evaluating recorded percentiles and resource usage.
   - Exit with code `1` if thresholds are violated.

4. **Integration with check.ps1**:
   - Add a step to `check.ps1` to execute a local standalone benchmark run (e.g. against the cache driver) and verify performance gates are met.
