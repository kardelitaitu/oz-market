# Design Decisions - SSE Mid-stream Cancellation

## Decision: Spawn task for async cancellation
- **Alternative 1**: Run synchronously and hope for some timeout. Not possible because `read_sse_stream` will loop infinitely on an open stream unless cancelled.
- **Alternative 2**: Spawn a background task using `tokio::spawn` and update the shared `AtomicBool` cancelled state from the parent task. This is the standard way to test cancellation of an async loop in Rust.
- **Choice**: **Alternative 2**. It is deterministic, matches runtime behavior, and is easily controlled via timers/sleeps.
