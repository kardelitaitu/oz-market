---
id: 0009-listener-status-serde-roundtrip-test
title: ListenerStatus Serde Round-trip Test
status: done
owner: mobile-team
implementer: agent
priority: P2
---

# ListenerStatus Serde Round-trip Test

Status: `done`
Implementer: `agent`

## Summary

This specification defines the requirements for adding a serialization and deserialization (serde) round-trip test for `ListenerStatus` in [sse.rs](file:///c:/My%20Script/project-the-marketplace/mobile/marketplace/src-tauri/src/client/sse.rs). This confirms that JSON data representing listener connection states round-trips correctly and is formatted as lower-cased strings as expected by the Tauri frontend.

## Scope

### In Scope
- Adding `serde::Deserialize` to the derives list of `ListenerStatus`.
- Implementing a unit test verifying that serializing variants results in the lowercase variant names (e.g. `"connecting"`, `"connected"`, `"reconnecting"`, `"disconnected"`, `"error"`).
- Testing JSON round-tripping of all `ListenerStatus` values.

### Out of Scope
- Rewriting `ListenerStatus::as_str()`.

## Proposed Direction
1. Update `ListenerStatus`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
   #[serde(rename_all = "lowercase")] // Ensure deserializer matches lowercase strings
   pub enum ListenerStatus { ... }
   ```
2. Write the unit test:
   - For each variant `v` of `ListenerStatus`:
     - Serialize `v` to JSON string `s`.
     - Confirm that `s` is equal to the expected lowercase string representation (surrounded by quotes, e.g. `"\"connecting\""`).
     - Deserialize `s` back to `ListenerStatus` and assert equality with `v`.
