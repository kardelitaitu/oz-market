# Backend Benchmark Suite

This document plans the design of a unified, production-grade CLI benchmarking suite tool for the backend. It provides a single entry point to stress test, identify bottlenecks, and measure performance limits across all critical infrastructure modules under realistic production loads.

---

## 1. Objectives

* **Single Entry Point**: Replace fragmented benchmark scripts with a single cohesive binary.
* **Continuous Integration Integration**: Output parseable performance stats (JSON format) to catch performance regressions in pipelines.
* **Component Isolation**: Allow targeting specific sub-systems (Postgres, Ledger Cache, WAL, SSE/events, HTTP router) or executing full system runs.
* **Production-Grade Telemetry**: Eliminate benchmarking biases (such as coordinated omission) and measure true tail-latencies.
* **Future-Proof Scalability**: Support distributed load generation and pluggable component drivers.

---

## 2. CLI Tool Architecture

We will implement this as a new binary crate/target in the backend codebase (e.g., `backend/server/src/bin/bench_suite.rs` or `backend/crates/bench-suite`):

```bash
# Standalone Local Mode (default)
cargo run --bin bench-suite -- --target cache --duration 10s

# Run as coordinator in a distributed cluster
cargo run --bin bench-suite -- --role coordinator --config profiles/smoke-test.yaml

# Run as worker node listening for coordinator commands
cargo run --bin bench-suite -- --role worker --coordinator-addr 10.0.0.1:50051
```

### Supported CLI parameters:
* `--role` (`standalone` | `coordinator` | `worker`) - Distributed execution role (default: standalone).
* `--config` - Path to declarative YAML profile (optional for standalone mode).
* `--target` (`all` | `postgres` | `cache` | `wal` | `sse` | `http`) - Target component (shorthand for standalone mode).
* `--concurrency` - Number of concurrent task workers.
* `--rate` - Fixed request rate (requests per second) for Coordinated Omission correction.
* `--duration` - Duration of the stress test (e.g., `10s`, `1m`).
* `--ramp-up` - Dynamically scale traffic load until the system saturates (breaks).
* `--output` (`text` | `json`) - Format of reporting details.

---

## 3. Local Development (Standalone Mode)

To ensure rapid developer iterations, the suite operates locally out-of-the-box without requiring complex setup:

* **Single-Process Execution**: By default, running with `--role standalone` (or omitting the role parameter) launches all generators and drivers in the same OS process.
* **Local Tokio Runtime**: The runner uses the local `tokio` multi-threaded scheduler to spawn concurrent worker tasks directly.
* **No Cluster Setup Required**: No gRPC servers, docker clusters, or multi-node configurations are needed. It communicates directly with local Postgres (`localhost:5432`) or memory caches.
* **Immediate Feedback**: Perfect for executing quick checks before committing code changes or validating performance gates locally.

---

## 4. Scalable & Future-Proof Architecture (Cluster Mode)

### A. Distributed Coordinator-Worker Topology
* **The Problem**: A load generator running on the same machine as the server steals CPU/RAM, deflating results. Running it on a single remote machine is limited by that machine's network card and core capacity.
* **The Solution**: Build a coordinator-worker model:
  * **Coordinator Node**: Parses the configuration profile, manages test phases, syncs worker starts via gRPC, aggregates HDR Histograms from all workers using serialization protocols, and generates the final JSON report.
  * **Worker Nodes**: Run on separate machines, receive rate/concurrency targets from the coordinator, generate raw traffic to target services, and stream metrics back.

```
       +--------------------+
       |    Coordinator     |
       +---------+----------+
                 | gRPC Sync
      +----------+----------+
      |                     |
+-----v------+        +-----v------+
|  Worker 1  |        |  Worker 2  |
+-----+------+        +-----+------+
      | Traffic             | Traffic
      +----------+----------+
                 |
        +--------v--------+
        | Backend Cluster |
        +-----------------+
```

### B. Driver-Based Extensibility (`BenchmarkDriver` Trait)
* **The Solution**: Define a generic trait that isolates target implementation details from the load generator engine:

```rust
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait BenchmarkDriver: Send + Sync {
    /// Initialize connection pools, target files, or network clients.
    async fn setup(&self) -> Result<(), BenchError>;

    /// Execute a single query or operation payload.
    async fn run_operation(&self) -> Result<Duration, BenchError>;

    /// Clean up files, connections, or state.
    async fn teardown(&self) -> Result<(), BenchError>;
}
```

### C. Declarative Test Profiles (Configuration-as-Code)
* **The Solution**: Store scenarios in YAML files. Example `profiles/heavy-write.yaml`:

```yaml
name: "Heavy Write Stress Test"
driver: "wal"
phases:
  - name: "Warmup"
    duration: "10s"
    rate: 1000
    concurrency: 4
  - name: "RampUp"
    duration: "30s"
    rate: 10000
    concurrency: 16
  - name: "PeakLoad"
    duration: "60s"
    rate: 50000
    concurrency: 64
thresholds:
  latency_p99_ms: 2.0
  error_rate_percent: 0.5
```

---

## 5. Advanced Benchmarking Methodologies

### A. Coordinated Omission Correction
* **The Solution**: The tool generates requests at a strict **fixed rate interval** (e.g., one request every 100 microseconds) regardless of when the previous requests returned. If the server stalls, requests back up in a queue, registering their queuing delay in the latency stats.

### B. High-Dynamic-Range (HDR) Histograms (`hdrhistogram` crate)
* **The Solution**: Use HDR Histograms to record latencies with configurable precision (e.g., 3 significant figures) across a huge dynamic range (1 microsecond to 60 seconds) to capture P99.9 and P99.99 outliers.

### C. Dynamic Saturation Ramping
* **The Solution**: Step up target rate by configured increments (e.g., 1000 QPS every 5 seconds). Stop ramping and record maximum stable capacity when error rate exceeds 1% or P95 latency exceeds the threshold.

### D. System Resource Correlation (`sysinfo` crate)
* **The Solution**: Background sampling of CPU utilization, RAM heap allocations, and disk I/O write bytes per second to correlate hardware constraints with request rates.

---

## 6. Benchmarking Targets

* **Database (`--target postgres`)**: Query latency, transaction commit speed, connection pool contention under load.
* **Dual-Layer Ledger Cache (`--target cache`)**: Read hit throughput, write-through lock contention.
* **Write-Ahead Log Logging (`--target wal`)**: Disk serialization speed, sequential file I/O latency.
* **Server-Sent Events (SSE) & Broadcast (`--target sse`)**: Broadcast channel dispatch lag and memory overhead.
* **HTTP API Layer (`--target http`)**: Rate limiter overhead, request routing latency.

---

## 7. Output Schema (JSON Mode)

```json
{
  "timestamp": "2026-06-05T09:07:00Z",
  "profile_name": "Heavy Write Stress Test",
  "system_info": {
    "cores": 8,
    "memory_bytes": 17179869184
  },
  "results": {
    "wal": {
      "throughput_qps": 85432.1,
      "latency_p50_ms": 0.08,
      "latency_p95_ms": 0.45,
      "latency_p99_ms": 1.2,
      "latency_p999_ms": 2.8,
      "coordinated_omission_corrected": true
    }
  },
  "resource_metrics": {
    "avg_cpu_percent": 64.2,
    "peak_memory_bytes": 268435456,
    "disk_write_mb_per_sec": 42.1
  }
}
```

---

## 8. Implementation Roadmap

### Phase 1: CLI Scaffolding & Setup
* **Crate Configuration**: Define the `bench-suite` binary target in `backend/server/Cargo.toml`.
* **Clap parsing**: Handle the `--role`, `--config`, and coordinator connection parameters.
* **Driver Trait**: Define the `BenchmarkDriver` trait and implement base error structures.

### Phase 2: Driver Implementations
* **Database Driver**: Concrete implementation of the trait for Postgres queries.
* **Cache Driver**: Implementation querying the `LedgerCache`.
* **WAL Driver**: Implementation flushing mock data to the WAL.

### Phase 3: Standalone Engine & Local Telemetry
* **Coordinated Omission Scheduler**: Implement an interval-based task scheduler that dispatches query tasks at regular, time-aligned intervals.
* **System Telemetry Collector**: Integrate `sysinfo` calls to capture CPU usage and memory metrics on a background thread during active runs.

### Phase 4: Distributed Coordinator-Worker Protocol
* **gRPC Integration**: Implement coordinator-worker gRPC communication to control start signals, update rates, and streams metrics.
* **Histogram Aggregation**: Implement serialization of HDR Histograms from workers and merge them on the coordinator.
