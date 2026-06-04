# Design Decisions - Reusable Counted Responder

- **Placement**: Keep it in the testing module of `sse.rs`. It does not need to go into a shared library module unless other integration tests outside of `sse.rs` start needing it.
- **Exhaustion Behavior**: Once all responses are consumed, returning a `500 Internal Server Error` is clean, standard, and stops reconnect loop behaviors in tests.
