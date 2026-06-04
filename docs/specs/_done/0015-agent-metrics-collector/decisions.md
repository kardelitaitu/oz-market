# Decisions - Agent Metrics Collector

## Architecture Decisions

### 1. In-Memory Sliding Window
- **Decision**: Metric samples will be collected in-memory within a sliding window (e.g., `VecDeque` protected by `Mutex` inside a `DashMap`).
- **Rationale**: Persisting every request duration to Postgres adds millisecond-level disk I/O latency to request dispatch paths, which violates sub-ms dispatch goals.

### 2. Cap capacity of queue
- **Decision**: Queue length is bounded to 100 samples per agent.
- **Rationale**: Prevents unbounded heap memory consumption for long-running servers.
