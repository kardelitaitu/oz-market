# Quality Rules - Benchmark Distributed Protocol

- **Robust Connection Handling**: Worker gRPC streams must implement auto-reconnect backoff strategies if the coordinator connection drops.
- **Clock Drift Mitigation**: Coordinator must calculate start time using absolute Epoch offsets, with workers verifying local clocks match within acceptable boundaries ($< 100$ms drift).
- **Graceful Aggregation**: Merging corrupted protobuf frames must not panic the coordinator thread, yielding warning events instead.
