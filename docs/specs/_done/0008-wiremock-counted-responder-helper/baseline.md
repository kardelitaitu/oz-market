# Current Baseline - Reusable Counted Responder

Currently, `sse.rs` has a stateful responder struct:
```rust
struct FirstTimeoutThenOk {
    count: AtomicUsize,
}
```
This is hardcoded to return a 5s delay on request 0, a specific SSE payload on request 1, and 500 thereafter. If we want to test other sequences (e.g. timeout -> reconnect -> success, or timeout -> all attempts timeout), we would have to implement a separate custom struct for each scenario.
