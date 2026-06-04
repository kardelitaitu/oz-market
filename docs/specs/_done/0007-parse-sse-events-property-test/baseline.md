# Current Baseline - Property Test for parse_sse_events

The codebase has unit tests for `parse_sse_events` inside `sse.rs` testing standard shapes (e.g. event with data, bare data, heartbeat, carriage returns, multiple messages).
However, it only tests static hand-crafted strings, leaving potential edge cases and boundary conditions unexercised.
