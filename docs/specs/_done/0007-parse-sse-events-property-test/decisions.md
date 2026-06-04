# Design Decisions - Property Test for parse_sse_events

- **Choice of tool**: We will use `proptest` since it is already used in `backend/server/Cargo.toml` and provides a very rich, macro-driven way to define properties in Rust.
- **Constraints on generated values**:
  - `event_type` cannot contain newlines.
  - `data` cannot contain newlines (`\n`) or carriage returns (`\r`). The current parser implementation does not support multi-line data fields (it overwrites `event_data` with the latest line instead of appending). To test round-trip parity without data loss, the data generation strategy must restrict characters to exclude `\n` and `\r`.

