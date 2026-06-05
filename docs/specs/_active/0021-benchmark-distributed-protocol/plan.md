# Plan - Benchmark Distributed Protocol

## Implementation Steps

1. **Protobuf Contract Definitions**:
   - Create `backend/server/proto/benchmark.proto`.
   - Declare `Coordinator` service with methods: `RegisterWorker`, `SyncStart`, `StreamMetrics`.

2. **gRPC Server & Client Setup**:
   - Configure gRPC compiler dependencies (`tonic`, `prost`) inside Cargo.toml.
   - Implement gRPC server on Coordinator.
   - Implement gRPC client connection handler on Worker.

3. **Metrics Serialization**:
   - Utilize `hdrhistogram::serializing::Serializer` to encode histograms to bytes.
   - Implement stream handler that periodically sends worker metrics to the coordinator.

4. **Merger Logic**:
   - Use `hdrhistogram::Histogram::add()` to merge received worker histograms.
