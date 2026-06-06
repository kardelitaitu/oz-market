# Parity Report — Benchmark Resource Profiling and CI Gating

| Item | Status | Details |
|------|--------|---------|
| sysinfo integration | ✅ **IMPLEMENTED** | `sysinfo = "0.32"` in `Cargo.toml`; `sysinfo::System` used in `ResourceMonitor` |
| Resource Monitor | ✅ **IMPLEMENTED** | `ResourceMonitor` struct in `resource_monitor.rs` — `start()` spawns OS thread, samples CPU (`sys.global_cpu_usage()`) and memory (`sys.used_memory()`) at 500ms intervals, `ResourceReport` with avg CPU, peak memory, sample count |
| JSON reports | ✅ **IMPLEMENTED** | `BenchmarkReport` struct in `bench/report.rs` — serializes config, latency percentiles (P50/P95/P99/P999 in µs + ms), error count, and `ResourceReport` to pretty-printed JSON; `write_report()` function writes to specified path, creates parent dirs automatically; wired into `run_standalone` and `run_coordinator` via `--report-file` CLI arg; 3 unit tests |
| Disk metrics (capacity + I/O) | ✅ **IMPLEMENTED** | `DiskMetrics` struct with `total_space_bytes`, `min_available_bytes` (sysinfo cross-platform) AND `total_read_bytes`, `total_written_bytes` (Windows `GetProcessIoCounters` FFI, per-process). Sampler captures baseline and delta for I/O counters. On non-Windows platforms, I/O fields are 0. Displayed in CLI stdout and JSON report. |
| Gating Checks (thresholds) | ❌ **PENDING** | No threshold comparison logic; no YAML threshold config; no non-zero exit code on threshold violation |
| check.ps1 Integration | ❌ **PENDING** | `check.ps1` does not invoke the benchmark suite; no benchmark gating step exists in the CI workflow |

## Acceptance Criteria Coverage

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Background collector utilizing sysinfo records CPU, Memory, and Disk metrics during runs | ✅ **IMPLEMENTED** | CPU via `sys.global_cpu_usage()`, memory via `sys.used_memory()`, disk capacity via `Disks::new_with_refreshed_list()` and `disk.total_space()` / `disk.available_space()`. Disk I/O via raw `GetProcessIoCounters` FFI on Windows (`total_read_bytes`/`total_written_bytes`). On non-Windows platforms disk I/O is 0 (requires OS-specific API implementation). |
| Performance thresholds evaluate metrics correctly, exiting with non-zero code on failures | ❌ **PENDING** | No threshold config, evaluation logic, or exit code handling exists |
| JSON reports are outputted matching the designated telemetry schema | ✅ **IMPLEMENTED** | `BenchmarkReport` with full schema (timestamp, target, rate, duration, concurrency, samples, errors, P50/P95/P99/P999 in µs + ms, `ResourceReport` with `DiskMetrics`); `write_report()` writes to file path from `--report-file`; 3 unit tests covering construction, JSON serialization, and file creation |
| Gating checks are integrated and executed successfully in local check workflows | ❌ **PENDING** | `check.ps1` runs cargo checks and tests but does not run benchmark suite or resource gating |

## Remaining Work

1. ✅ **JSON report output** — done with `BenchmarkReport` + `write_report()` + `--report-file`
2. ✅ **Disk metrics (capacity + I/O)** — capacity via sysinfo (cross-platform), I/O via `GetProcessIoCounters` FFI (Windows). Non-Windows platforms report 0 for I/O fields.
3. **Threshold configuration** — define a YAML schema for CPU/memory/disk thresholds; parse and evaluate after benchmark
4. **Check.ps1 integration** — add an optional benchmark gating step in `check.ps1` (guarded by a flag like `-BenchGating`)

## Files

- `backend/server/src/bench/resource_monitor.rs` — `ResourceMonitor` (implemented), `ResourceReport` (implemented)
- `backend/server/Cargo.toml` — `sysinfo = "0.32"` dependency (present)
- `backend/server/src/bin/bench_suite.rs` — uses `ResourceMonitor` in `run_standalone` (present)
- `check.ps1` — no benchmark gating step (missing)
