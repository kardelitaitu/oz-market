# Decisions - Benchmark Distributed Protocol

## Architecture Decisions

### 1. gRPC over Tonic for Node Coordination
- **Decision**: Use `tonic` (gRPC) for clustering and nodes control synchronization.
- **Rationale**: Tonic provides strong typed contracts, low latency communication, and efficient HTTP/2 asynchronous streaming capabilities suited for high-frequency telemetry transport.

### 2. HDR Histogram Merging
- **Decision**: Workers will serialize local histograms and transmit them, rather than streaming raw transaction data points. The coordinator will merge them using the `add()` API.
- **Rationale**: Drastically reduces network bandwidth overhead during benchmarking runs (bytes are size-bounded, whereas streaming individual operation durations grows linearly with QPS).
