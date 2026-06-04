# TODO — Infrastructure Hardening Roadmap

Takes `oz-market` from highly optimized open-source monorepo to bulletproof production-grade infrastructure.

Ordered by feasibility + codebase readiness.

---

## ✅ Phase 1 — Non-Blocking Timeout Guardrails

> Protect sub-ms server perf from slow agent calls with hard execution deadlines.

**Completed 2026-06-04:**

- [x] Wrap MCP tool handlers with `tokio::time::timeout()` — configurable per-tool deadline via `MCP_TOOL_TIMEOUT_MS` env var (default 10s)
- [x] Return `McpToolError::timeout()` with code `"timeout"` on timeout
- [x] Add timeout metadata to `ServerInfo.instructions` in `get_info()`
- [x] Audit `api-contract`: no `deny_unknown_fields` anywhere; all optional fields have `#[serde(default)]`
- [x] Log internal errors and timeouts to stderr via `eprintln!` (respects MCP stderr convention)
- [x] Document rollback boundary: idempotency keys already prevent duplicate mutations on timeout+retry

---

## Phase 2 — Resilient Mobile Integration

> Replace aggressive polling with server-push model; reduce battery drain without sacrificing responsiveness.

- [ ] Add SSE endpoint on server for negotiation status changes (`GET /v1/events/negotiations`)
- [ ] Wire Tauri events system to consume SSE stream (`listen()` API)
- [ ] Remove 5s polling from `negotiations/[id]/+page.svelte` — replace with SSE-driven reactivity
- [ ] Reduce rate-limit polling from 3s to 15s (acceptable staleness for rate-limit UI)
- [ ] Add state reconciliation on reconnect — fetch latest state after SSE reconnection
- [ ] Measure and document JSON serialization cost at FFI boundary; set concrete threshold for zero-copy reconsideration (e.g. >5ms per call)
- [ ] Cap exponential backoff at 30s with jitter on mobile reconnect

---

## Phase 3 — Dual-Layer Ledger

> Track credits, rate limits, and auth tokens in-memory; async batch commit to Postgres.

**Prerequisite: define and build the credit/balance model first (schema, deposit/spend).**

- [ ] Design credit/balance DB schema and domain logic
- [ ] Implement in-memory ledger trait (dashmap or embedded Redis) — same pattern as existing `SlidingWindowRateLimiter`
- [ ] Define cache invalidation policy (TTL + admin-triggered invalidation)
- [ ] Start with synchronous commit (credits written to DB immediately via the cache as write-through); add async batch only after proving the bottleneck
- [ ] Add WAL or write-ahead log for crash recovery if moving to async batch
- [ ] Add `cache_hit` / `cache_miss` / `batch_lag` metrics
- [ ] Benchmark contention improvement vs direct Postgres writes
- [ ] Note: existing single-process HashMap cache does not scale horizontally — cross-instance coordination needs Redis

---

## Phase 4 — Predictive Latency Scoring (Backlog)

> Dynamic agent health metrics as a routing signal.

**Blocked on: multi-agent routing/dispatch layer does not exist yet.**

- [ ] Deferred — requires building an agent-pool / dispatch system first
- [ ] Define latency + error-rate scoring model (EWMA, percentile-based)
- [ ] Collect per-call metrics: response time, failure rate, windowed averages
- [ ] Solve cold-start problem (probationary default score for new agents)
- [ ] Expose health status in API contract for clients
- [ ] Circuit-breaker integration — skip chronically slow agents

---

## Phase 5 — Semantic Tool Versioning (Backlog)

> Multi-version contract support in `api-contract` crate.

**Deferred — premature. Existing serde defaults + `schema_version` field cover current needs.**

- [ ] Add warning log when unrecognized `schema_version` is received
- [ ] Defer multi-version parsing until backward-compatibility pain is confirmed
- [ ] If revisited: define version negotiation handshake (client advertises range, server responds)
- [ ] If revisited: define deprecation window and migration policy per version
