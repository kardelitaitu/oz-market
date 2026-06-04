---
id: 0008-wiremock-counted-responder-helper
title: Wiremock Reusable Counted Responder Helper
status: done
owner: testing-team
implementer: agent
priority: P2
---

# Wiremock Reusable Counted Responder Helper

Status: `done`
Implementer: `agent`

## Summary

This specification outlines the requirements to create a reusable helper struct `CountedResponder` and its associated factory function `counted_responder(responses: Vec<ResponseTemplate>)` inside the integration test module of [sse.rs](file:///c:/My%20Script/project-the-marketplace/mobile/marketplace/src-tauri/src/client/sse.rs). This avoids repeating stateful responder boilerplate (like `FirstTimeoutThenOk`) for multi-request test scenarios.

## Scope

### In Scope
- Designing a thread-safe `CountedResponder` that wraps a sequence of wiremock `ResponseTemplate`s.
- Implementing `wiremock::Respond` for `CountedResponder` to serve templates in sequence.
- Creating a `counted_responder` helper constructor.

### Out of Scope
- Creating a separate crate for test helpers. Scoping to the test suite of `sse.rs` is preferred unless broader utility is needed.

## Proposed Direction
1. Define the struct:
   ```rust
   struct CountedResponder {
       responses: Vec<ResponseTemplate>,
       count: std::sync::atomic::AtomicUsize,
   }
   ```
2. Implement `wiremock::Respond` so that `respond(&self, _request: &Request)` reads the current value, increments it, and returns the corresponding response template from `responses`. If the index is out of bounds, return `ResponseTemplate::new(500)`.
3. Provide the constructor:
   ```rust
   fn counted_responder(responses: Vec<ResponseTemplate>) -> CountedResponder {
       CountedResponder {
           responses,
           count: std::sync::atomic::AtomicUsize::new(0),
       }
   }
   ```
