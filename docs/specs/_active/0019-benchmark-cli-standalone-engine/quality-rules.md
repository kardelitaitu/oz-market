# Quality Rules - Benchmark CLI and Standalone Engine

- **High Precision Timing**: Avoid standard sleep timers for high QPS rates (e.g. $> 10000$ QPS), using high-resolution monotonic sleep hooks to minimize timing drift.
- **Accurate Coordinated Omission Recording**: Latency must be calculated relative to the scheduled request tick rather than the actual start time of the task.
- **Panic Protection**: Worker threads must catch internal panics gracefully, ensuring the runner does not freeze or leak memory during crashes.
