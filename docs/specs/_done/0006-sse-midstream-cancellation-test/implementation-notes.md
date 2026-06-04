# Implementation Notes - SSE Mid-stream Cancellation

- Use a custom stateful wiremock responder (or similar mock setup) to output chunks.
- Ensure the delay is small (e.g. 50ms) to keep unit tests fast.
- **Critical Strategy Note**: `read_sse_stream` checks the `cancelled` flag at the start of each loop iteration. It *cannot* check the flag while blocked awaiting a chunk inside `response.chunk().await`. Therefore, to prevent the test from hanging indefinitely, the mock responder must either write a second event or close the connection/stream after the cancellation flag is toggled, which forces the `chunk().await` future to resolve and trigger the loop check.
- The `cancelled` variable is passed as a reference: `Arc<AtomicBool>`. We can clone the arc and call `cancelled.store(true, Ordering::Relaxed)` from the main testing thread.

