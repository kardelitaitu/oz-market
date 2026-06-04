# Baseline - Predictive Latency Scoring

## Current State

As of starting Phase 4:
- The backend lacks any mathematical scoring or performance model for routing queries across multiple agents.
- No EWMA models exist to weight recent latencies or error rates.
- No probationary period or cold-start parameters exist for dynamic agents.
