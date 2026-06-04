---
id: 0006-sse-midstream-cancellation-test
title: SSE Mid-stream Cancellation Integration Test
status: done
owner: mobile-team
implementer: agent
priority: P2
---

# SSE Mid-stream Cancellation Integration Test

Status: `done`
Implementer: `agent`

## Summary

This specification describes the requirements for adding an integration test to verify the mid-stream cancellation behavior of `read_sse_stream` inside [sse.rs](file:///c:/My%20Script/project-the-marketplace/mobile/marketplace/src-tauri/src/client/sse.rs). 

## Scope

### In Scope
- A new integration test in `sse.rs` named `read_sse_stream_midstream_cancellation` (or similar).
- Verification that when `cancelled` is set to `true` while the stream is open and transmitting data, the loop terminates immediately.
- Verification that events emitted before cancellation are received, while subsequent events are ignored.

### Out of Scope
- Modifying production client or server code.
- Enhancing Tauri window event propagation.

## Proposed Direction
1. Launch `MockServer`.
2. Configure a responder that yields chunked response body data with delays (e.g. "event: first\ndata: 1\n\n", followed by a delay, then "event: second\ndata: 2\n\n").
3. Call `read_sse_stream` in a spawned asynchronous task.
4. Delay execution shortly, then set the cancellation `AtomicBool` to `true`.
5. Verify that only the first event is recorded by the collector.
