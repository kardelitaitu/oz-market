# Decisions - Benchmark Component Drivers

## Architecture Decisions

### 1. Driver-based Extensibility
- **Decision**: Code all benchmark targets as implementations of the async `BenchmarkDriver` trait.
- **Rationale**: Isolates task generation logic from the target services, allowing new components to be added without modifying core scheduling engine code.

### 2. Auto-cleaning Temp Files in WAL Benchmark
- **Decision**: The WAL driver must create temporary logging files in a sandbox directory and automatically delete them during the `teardown` call.
- **Rationale**: Prevents disk space leakage or stale file handles from building up on developer machines.
