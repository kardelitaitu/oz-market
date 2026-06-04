# Quality Rules - Agent Metrics Collector

- **O(1) Record Time**: Pushing new metric samples must be a constant-time operation ($O(1)$) and not trigger vector copies or array allocations.
- **Granular Locking**: Mutex guards must be placed on the individual agent queues rather than locking the entire `AgentMetricsCollector` table, allowing concurrent metrics collection for different agents.
- **Clock Safety**: All timestamps must use monotonic clocks (`Instant`) to avoid clock skew adjustments impacting duration measurements.
