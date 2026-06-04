# Decisions - Predictive Latency Scoring

## Architecture Decisions

### 1. Exponentially Weighted Moving Average (EWMA) for Scoring
- **Decision**: Use EWMA to calculate metrics rather than simple windowed arithmetic averages or raw percentiles.
- **Rationale**: EWMA only requires keeping the aggregated value in-memory, requiring $O(1)$ space, and adapts dynamically to recent agent degradation without lagging.

### 2. Probationary Cold-Start Baseline
- **Decision**: Assign a baseline of 200ms latency and 0% error rate for newly registered agents until they acquire at least 5 telemetry samples.
- **Rationale**: Prevents overloading new agents or starving them due to lack of metrics.
