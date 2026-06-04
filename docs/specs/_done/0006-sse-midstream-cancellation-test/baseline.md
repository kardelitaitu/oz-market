# Current Baseline - SSE Mid-stream Cancellation

Currently, `sse.rs` has several integration tests:
- `read_sse_stream_forwards_single_event`
- `read_sse_stream_forwards_multiple_events`
- `listen_negotiation_impl_emits_error_on_bad_status`
- `listen_negotiation_impl_emits_disconnected_when_cancelled_early`
- `listen_negotiation_impl_retries_after_timeout_then_succeeds`

However, none of these tests evaluate mid-stream cancellation where the cancellation token transitions to `true` while `read_sse_stream` is actively waiting on or processing network chunks.
