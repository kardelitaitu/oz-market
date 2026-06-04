---
id: 0007-parse-sse-events-property-test
title: Property Test for parse_sse_events
status: done
owner: mobile-team
implementer: agent
priority: P2
---

# Property Test for parse_sse_events

Status: `done`
Implementer: `agent`

## Summary

This specification defines requirements for introducing a property-based test for the `parse_sse_events` function inside [sse.rs](file:///c:/My%20Script/project-the-marketplace/mobile/marketplace/src-tauri/src/client/sse.rs). This property test ensures that all valid combinations of SSE events can be serialized and parsed back without losing metadata or payload data.

## Scope

### In Scope
- Adding `proptest` as a dev-dependency to the mobile app crate.
- Implementing a property test generating random alphanumeric strings for `event_type` and arbitrary non-newline/valid strings for `data`.
- Round-tripping the generated properties by formatting them to SSE format and asserting that `parse_sse_events` returns identical structs.

### Out of Scope
- Rewriting the parsing logic itself unless parser bugs are uncovered by the property test.
- Property testing other network functions.

## Proposed Direction
1. Add `proptest` dev-dependency.
2. Formulate generator strategies for:
   - `event_type`: Alphanumeric, optionally empty.
   - `data`: Arbitrary text containing no double-newlines (since double newlines mark message boundaries) and no lines starting with `data: ` inside the data string itself unless formatted/escaped. Alternatively, standard JSON payloads.
3. Combine them to construct the input string:
   ```rust
   let sse_block = format!("event: {}\ndata: {}\n\n", event_type, data);
   ```
4. Verify that calling `parse_sse_events(&sse_block)` parses exactly one message with matching properties.
