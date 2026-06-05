# Decisions - Benchmark CLI and Standalone Engine

## Architecture Decisions

### 1. Unified Binary Crate
- **Decision**: Define the benchmark suite as a `bin` target within the main server package rather than a separate repository.
- **Rationale**: Reuses database drivers, configuration loaders, and existing dependencies without duplicate configurations.

### 2. Microsecond Precision for Latency
- **Decision**: Latency samples will be stored in HDR Histograms with microsecond resolution.
- **Rationale**: Cache hits operate in sub-millisecond ranges (1-20 microseconds). Millisecond-level buckets would lose precision for cache performance analysis.
