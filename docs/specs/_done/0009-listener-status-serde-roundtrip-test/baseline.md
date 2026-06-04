# Current Baseline - ListenerStatus Serde Round-trip

Currently, `ListenerStatus` in `sse.rs` is defined as:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ListenerStatus {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
    Error,
}
```
And it has a helper `as_str()` function to get a lowercase representation manually for Actix payload emitting:
```rust
impl ListenerStatus {
    fn as_str(&self) -> &'static str {
        match self {
            ListenerStatus::Connecting => "connecting",
            ...
        }
    }
}
```
No deserialize support exists, and no test verifies that serializing a variant yields the exact lowercase string, or that a lowercase string deserializes back cleanly.
