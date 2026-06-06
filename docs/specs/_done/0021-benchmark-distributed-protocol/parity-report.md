# Parity Report — Benchmark Distributed Protocol

| Item | Status | Details |
|------|--------|---------|
| Protobuf contract | ✅ **IMPLEMENTED** | `proto/benchmark.proto` — 3 RPCs: `RegisterWorker` (unary), `SyncStart` (unary), `StreamMetrics` (client-side streaming) |
| gRPC Server (tonic) | ✅ **IMPLEMENTED** | `distributed.rs` — `CoordinatorService` implementing `proto::coordinator_server::Coordinator` trait, `#[tonic::async_trait]`, spawned as background task via `tokio::spawn` (not `select!`) |
| gRPC Client | ✅ **IMPLEMENTED** | `WorkerClient` — connects via `CoordinatorClient::connect()`, registers, syncs start time, streams histogram via `stream_metrics()` |
| Stream Metrics | ✅ **IMPLEMENTED** | Worker serializes histogram via `V2Serializer` and streams `MetricFrame` (worker_id, sequence_number, serialized_histogram bytes); coordinator deserializes and merges |
| Histogram merging | ✅ **IMPLEMENTED** | Coordinator uses `hdrhistogram::serialization::Deserializer` to deserialize each frame, then `state.histogram.add(remote_hist)` to merge; corrupt frames logged via `eprintln` (no panic) |
| Clock drift mitigation | ✅ **IMPLEMENTED** | `sync_start` RPC checks `|now - start_time_epoch_ms| < 100ms`, returns `Status::deadline_exceeded` on violation |
| Vendored protoc | ✅ **IMPLEMENTED** | `build.rs` uses `protoc_bin_vendored::protoc_bin_path()` with graceful `if let Ok` fallback; `tonic_build::compile_protos("proto/benchmark.proto")` |
| Coordinator runner | ✅ **IMPLEMENTED** | `run_coordinator()` spawns gRPC server, waits for N workers, sets synchronized start time via `Notify`, sleeps for benchmark duration, prints merged histogram |
| Worker runner | ✅ **IMPLEMENTED** | `run_worker()` connects, registers, syncs, creates driver, runs scheduler, streams histogram via `run_and_stream()` |
| Unit tests | ✅ **IMPLEMENTED** | 3 tests: coordinator service creation, histogram serialization roundtrip, histogram merge — all pass |

## Acceptance Criteria Coverage

| Criterion | Status | Evidence |
|-----------|--------|----------|
| gRPC service interfaces sync start states and streaming metrics between nodes successfully | ✅ | `Coordinator` proto with `SyncStart` (clock drift check + `Notify`-based synchronization) and `StreamMetrics` (client-side streaming of serialized histograms) |
| Workers connect to the coordinator gRPC server under the worker role configurations | ✅ | `--role worker --coordinator-addr 127.0.0.1:50051` registers with `RegisterWorker`, gets `assigned_id`, syncs via `SyncStart`, then streams results |
| Coordinator successfully deserializes and merges HDR Histograms from multiple worker streams | ✅ | `Deserializer::deserialize()` per `MetricFrame`, `state.histogram.add()` merges, graceful error handling for corrupt frames |

## Files

- `backend/server/proto/benchmark.proto` — protobuf service definition
- `backend/server/build.rs` — vendored protoc + tonic build
- `backend/server/src/bench/distributed.rs` — gRPC server, client, coordinator runner, +3 tests
- `backend/server/src/bin/bench_suite.rs` — `--role coordinator` / `--role worker` CLI wiring
