# Implementation Plan - Reusable Counted Responder

## Phase 1: Define Struct
- Add `CountedResponder` struct to `mobile/marketplace/src-tauri/src/client/sse.rs` inside the `mod integration` block.
- Equip the struct with a list of `ResponseTemplate`s and an `AtomicUsize`.

## Phase 2: Implement Respond Trait
- Implement the `wiremock::Respond` trait for `CountedResponder`.
- Verify thread safety (it must use `.fetch_add(1, Ordering::SeqCst)` or `Relaxed`).
- Return a 500 template for requests out of bounds.

## Phase 3: Integrate with Existing Tests
- Refactor any suitable tests to use `counted_responder` to verify correctness.
- Add helper docs explaining usage.
