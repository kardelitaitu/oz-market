# Validation Checklist - Benchmark Resource Profiling and CI Gating

This checklist is used to confirm the completion of Spec 0022:

- [ ] `sysinfo` dependency is declared in `backend/server/Cargo.toml`.
- [ ] Telemetry background thread launches, samples metrics, and aggregates avg CPU and peak memory correctly.
- [ ] Runner exits with error code `1` if a configured performance threshold is exceeded.
- [ ] Structured JSON report maps metrics correctly to output schema.
- [ ] Gating checks are integrated and pass locally via `check.ps1`.
