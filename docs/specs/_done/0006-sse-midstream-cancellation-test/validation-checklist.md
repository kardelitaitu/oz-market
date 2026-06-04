# Validation Checklist - SSE Mid-stream Cancellation

- [ ] New test case compiles cleanly in `mobile/marketplace/src-tauri/src/client/sse.rs`.
- [ ] Running `cargo test --manifest-path mobile/marketplace/src-tauri/Cargo.toml` executes the new test and passes.
- [ ] Setting `cancelled = true` is proven to terminate the loop (the test completes and does not hang).
- [ ] Captured events match expectations (only events prior to cancellation are captured).
