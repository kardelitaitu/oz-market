# Validation Checklist - Agent Metrics Collector

This checklist is used to confirm the completion of Spec 0015:

- [ ] `AgentMetricsCollector` is defined in `backend/server/src/services/agent_metrics.rs`.
- [ ] Recording samples updates the corresponding agent queue in the metrics store.
- [ ] Ring buffer capacity is strictly enforced, evicting the oldest sample if capacity is exceeded.
- [ ] Concurrent tests verify that concurrent metric writers do not cause race conditions or corrupt the sliding queues.
