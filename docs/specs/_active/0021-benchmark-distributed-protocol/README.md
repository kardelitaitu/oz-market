---
id: 0021-benchmark-distributed-protocol
title: Benchmark Distributed Protocol
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Benchmark Distributed Protocol

Status: `active`
Implementer: `agent`

## Summary

This specification governs the coordinator-worker distributed protocol configuration. It enables multi-node synchronized load testing using gRPC communication channels and decentralized histogram aggregation.

## Scope

### In Scope
- Declaring gRPC service definitions (`proto/benchmark.proto`) for coordinator-worker nodes.
- Synchronizing test starting times across all registered workers.
- Serializing and streaming HDR Histogram samples over gRPC.
- Implementing the histogram merging logic on the coordinator.

### Out of Scope
- Local single-process task scheduling (managed by Spec 0019).
- Hardware resource monitors (deferred to Spec 0022).

## Proposed Direction
1. Communication Protocol:
   - Use `tonic` for gRPC implementation.
   - Define start and progress sync messages.
2. Histogram Serialization:
   - Use `hdrhistogram` built-in serializers to convert local histograms to byte streams for transmission.
   - Combine worker histograms into a unified coordinator report.
