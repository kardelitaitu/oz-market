# Plan - Predictive Latency Scoring

## Implementation Steps

1. **EWMA Implementation**:
   - Create `backend/server/src/services/latency_scorer.rs`.
   - Define a `LatencyScorer` service.
   - Implement mathematical calculation for EWMA: `calculate_ewma(new_val: f64, prev_ewma: f64, alpha: f64) -> f64`.

2. **Agent Score Calculator**:
   - Define `AgentScore` struct containing `ewma_latency` (in ms) and `ewma_error_rate` (0.0 to 1.0).
   - Implement scoring logic over telemetry samples from `AgentMetricsCollector`.
   - Provide a dynamic default fallback on cold start (no samples):
     - Configurable default EWMA Latency: 200.0 ms.
     - Configurable default EWMA Error Rate: 0.0.
