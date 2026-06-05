# Decisions - Benchmark Resource Profiling and CI Gating

## Architecture Decisions

### 1. Simple sysinfo Sampling Rate (500ms)
- **Decision**: Hardware metrics will be sampled every 500ms in a dedicated OS thread.
- **Rationale**: Keeps execution overhead extremely low ($< 0.1\%$ CPU overhead) to prevent telemetry collection from impacting the validity of the benchmark results.

### 2. Exit Status Codes for CI Integration
- **Decision**: Return exit code `0` on performance target compliance, and exit code `1` (or others) on latency/error threshold breaches.
- **Rationale**: Matches POSIX pipeline status standards, allowing immediate integration with GitHub Actions workflows and local execution scripts (`check.ps1`).
