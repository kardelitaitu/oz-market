# Implementation Plan - ListenerStatus Serde Round-trip

## Phase 1: Derive Deserialize and Add Rename Attribute
- Modify `ListenerStatus` in `mobile/marketplace/src-tauri/src/client/sse.rs` to derive `serde::Deserialize`.
- Use the `#[serde(rename_all = "lowercase")]` attribute to match lowercased representations.

## Phase 2: Implement Unit Test
- Implement `test_listener_status_serde_roundtrip` inside `sse.rs`'s `tests` module.
- Iterate over all variants, serializing each variant to JSON and asserting that it matches the lowercased string format.
- Deserialize it back and assert it matches the original variant.
