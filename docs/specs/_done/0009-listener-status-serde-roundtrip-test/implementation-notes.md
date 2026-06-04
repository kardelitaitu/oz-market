# Implementation Notes - ListenerStatus Serde Round-trip

Deriving `serde::Deserialize` requires importing or prefixing it as `serde::Deserialize`.
Update the declaration:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListenerStatus {
    ...
}
```
In `test_listener_status_serde_roundtrip`:
```rust
let variants = vec![
    (ListenerStatus::Connecting, "\"connecting\""),
    (ListenerStatus::Connected, "\"connected\""),
    (ListenerStatus::Reconnecting, "\"reconnecting\""),
    (ListenerStatus::Disconnected, "\"disconnected\""),
    (ListenerStatus::Error, "\"error\""),
];
for (var, expected_json) in variants {
    let serialized = serde_json::to_string(&var).unwrap();
    assert_eq!(serialized, expected_json);
    let deserialized: ListenerStatus = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, var);
}
```
