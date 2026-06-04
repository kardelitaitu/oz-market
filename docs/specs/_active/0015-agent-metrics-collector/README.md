---
id: 0015-agent-metrics-collector
title: Agent Metrics Collector
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Agent Metrics Collector

Status: `active`
Implementer: `agent`

## Summary

This specification defines the design of the in-memory agent telemetry metrics collector. It tracks recent query response times (latencies) and failure events for each agent within a thread-safe sliding window buffer.

## Scope

### In Scope
- A thread-safe metrics storage service (`AgentMetricsCollector`) that retains the last $N$ query samples.
- Recording query latency (duration) and success/failure status of dispatch actions.
- Storing sample data in-memory without database write overhead.

### Out of Scope
- Calculating predictive scores or updating EWMA states (deferred to Spec 0016).
- Circuit breaker state transitions (deferred to Spec 0017).

## Proposed Direction
1. In-Memory Ring/Sliding Window Buffer:
   - For each active agent, store a queue of telemetry samples containing a timestamp, duration, and success status.
   - Cap the queue size (e.g., 100 samples) to prevent memory leaks.
2. Concurrent Telemetry Collection:
   - Provide a method `record_sample(agent_id, duration, is_success)` called after agent dispatches.
