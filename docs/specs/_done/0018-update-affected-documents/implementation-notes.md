# Implementation Notes - Update Affected Documents

## Updating `TODO.md` Checklist

Mark the following checklist items in `TODO.md` as complete:
- [x] Add SSE endpoint on server for negotiation status changes (`GET /v1/events/negotiations`)
- [x] Wire Tauri events system to consume SSE stream (`listen()` API)
- [x] Remove 5s polling from `negotiations/[id]/+page.svelte` — replace with SSE-driven reactivity
- [x] Reduce rate-limit polling from 3s to 15s (acceptable staleness for rate-limit UI)
- [x] Add state reconciliation on reconnect — fetch latest state after SSE reconnection
- [x] Measure and document JSON serialization cost at FFI boundary; set concrete threshold for zero-copy reconsideration (e.g. >5ms per call)
- [x] Cap exponential backoff at 30s with jitter on mobile reconnect

## Indexing Specifications

Add the following entries under the active/done specifications tables in `docs/specs/README.md` and related index files:
- Spec 0014: Agent Routing and Dispatch Core Layer
- Spec 0015: Agent Metrics Collector
- Spec 0016: Predictive Latency Scoring
- Spec 0017: Agent Circuit-Breaker and Health API
- Spec 0018: Update Affected Documents
