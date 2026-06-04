# Baseline - Agent Metrics Collector

## Current State

As of starting Phase 4:
- The backend does not collect any latency, response time, or failure status metrics for individual agents.
- Queries are executed without any telemetry tracking or logging of execution timings.
- There is no in-memory sliding window or metric ring-buffer service.
