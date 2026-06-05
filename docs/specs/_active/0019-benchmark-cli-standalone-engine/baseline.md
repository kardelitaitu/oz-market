# Baseline - Benchmark CLI and Standalone Engine

## Current State

As of starting Phase 4:
- The backend lacks any unified benchmarking binary target; performance checks are written in separate temporary scripts.
- No CLI parameters exist to specify benchmark runs, durations, or concurrency.
- Coordinated omission correction is not implemented, causing latencies to be biased during server load spikes.
- No HDR Histogram metric collection exists for tail latencies.
