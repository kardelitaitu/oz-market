# Quality Rules - Predictive Latency Scoring

- **Deterministic Decay**: EWMA must use a consistent decay factor (alpha) to ensure score changes are predictable and mathematical bounds are preserved.
- **Float Bounds Checking**: Recalculated EWMA values must be validated to prevent `NaN` or `Infinite` states under extreme test inputs.
- **Zero Alloc Scoring**: Score calculation must run entirely in-place over the metrics window slice, without allocating memory or duplicating lists.
