# Plan - Agent Metrics Collector

## Implementation Steps

1. **Telemetry Struct Definition**:
   - Create `backend/server/src/services/agent_metrics.rs`.
   - Define `AgentTelemetrySample` holding timestamp, duration, and success status.

2. **Metrics Collection Engine**:
   - Implement `AgentMetricsCollector` using a `DashMap<Uuid, std::sync::Mutex<std::collections::VecDeque<AgentTelemetrySample>>>` or similar thread-safe concurrent design.
   - Enforce a maximum window capacity (e.g. `CAPACITY = 100`).
   - Implement `record_sample(agent_id: Uuid, duration: std::time::Duration, is_success: bool)` which pushes a sample and evicts the oldest if capacity is exceeded.
   - Implement `get_samples(agent_id: &Uuid) -> Vec<AgentTelemetrySample>` to retrieve samples for analysis.
