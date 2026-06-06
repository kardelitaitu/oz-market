# Implementation Notes - Benchmark Distributed Protocol

## Protobuf Definition and gRPC Schema

Below is the gRPC contract design for multi-node benchmark synchronization:

```protobuf
syntax = "proto3";
package benchmark;

service Coordinator {
    // Register worker with the coordinator node
    rpc RegisterWorker (RegisterRequest) returns (RegisterResponse);

    // Sync execution parameters and unified start time
    rpc SyncStart (SyncRequest) returns (SyncResponse);

    // Stream serialized HDR Histograms from worker to coordinator
    rpc StreamMetrics (stream MetricFrame) returns (MetricResult);
}

message RegisterRequest {
    string worker_id = 1;
    string worker_address = 2;
}

message RegisterResponse {
    bool accepted = 1;
    string assigned_id = 2;
}

message SyncRequest {
    string config_yaml = 1;
    int64 start_time_epoch_ms = 2;
}

message SyncResponse {
    bool ready = 1;
}

message MetricFrame {
    string worker_id = 1;
    int64 sequence_number = 2;
    bytes serialized_histogram = 3;
}

message MetricResult {
    bool success = 1;
}
```

## Serializing and Deserializing Histograms in Rust

```rust
use hdrhistogram::Histogram;
use hdrhistogram::serialization::V2Serializer;
use hdrhistogram::serialization::Deserializer;

pub fn serialize_histogram(hist: &Histogram<u64>) -> Result<Vec<u8>, String> {
    let mut serializer = V2Serializer::new();
    let mut buf = Vec::new();
    serializer
        .serialize(hist, &mut buf)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn deserialize_and_merge(
    target_hist: &mut Histogram<u64>,
    payload: &[u8],
) -> Result<(), String> {
    let remote_hist = Deserializer::new()
        .deserialize(&mut &payload[..])
        .map_err(|e| e.to_string())?;
    target_hist.add(remote_hist).map_err(|e| e.to_string())?;
    Ok(())
}
```
