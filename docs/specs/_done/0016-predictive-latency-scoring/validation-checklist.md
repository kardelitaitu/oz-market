# Validation Checklist - Predictive Latency Scoring

This checklist is used to confirm the completion of Spec 0016:

- [ ] `LatencyScorer` is implemented in `backend/server/src/services/latency_scorer.rs`.
- [ ] Scoring formulas for EWMA latency and error rates are verified mathematically in unit tests.
- [ ] Newly registered agents without telemetry default to the configured cold-start baseline (200ms latency, 0% error rate).
- [ ] Score recalculates correctly as new success/failure samples are recorded.
