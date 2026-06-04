# Validation Checklist - ListenerStatus Serde Round-trip

- [ ] `ListenerStatus` implements `serde::Deserialize`.
- [ ] New unit test `test_listener_status_serde_roundtrip` added inside `tests` block in `sse.rs`.
- [ ] All variants correctly serialize to lowercase string.
- [ ] All variants correctly deserialize back to variants.
