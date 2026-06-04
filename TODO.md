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

## ✅ Phase 2 — Resilient Mobile Integration

> Replace aggressive polling with server-push model; reduce battery drain without sacrificing responsiveness.

**Completed 2026-06-05:**

- [x] Add SSE endpoint on server for negotiation status changes (`GET /v1/events/negotiations`)
- [x] Wire Tauri events system to consume SSE stream (`listen()` API)
- [x] Remove 5s polling from `negotiations/[id]/+page.svelte` — replace with SSE-driven reactivity
- [x] Reduce rate-limit polling from 3s to 15s (acceptable staleness for rate-limit UI)
- [x] Add state reconciliation on reconnect — fetch latest state after SSE reconnection
- [x] Measure and document JSON serialization cost at FFI boundary; set concrete threshold for zero-copy reconsideration (e.g. >5ms per call)
- [x] Cap exponential backoff at 30s with jitter on mobile reconnect

---

## ✅ Phase 3 — Dual-Layer Ledger

> Track credits, rate limits, and auth tokens in-memory; async batch commit to Postgres.

**Completed 2026-06-05:**

- [x] Design credit/balance DB schema and domain logic
- [x] Implement in-memory ledger trait (dashmap) — same pattern as existing `SlidingWindowRateLimiter`
- [x] Define cache invalidation policy (TTL + admin-triggered invalidation)
- [x] Start with synchronous commit (credits written to DB immediately via the cache as write-through); add async batch only after proving the bottleneck
- [x] Add WAL or write-ahead log for crash recovery if moving to async batch
- [ ] Add `cache_hit` / `cache_miss` / `batch_lag` metrics *(deferred)*
- [ ] Benchmark contention improvement vs direct Postgres writes *(deferred)*
- [ ] Note: existing single-process HashMap cache does not scale horizontally — cross-instance coordination needs Redis

---

## ✅ Phase 4 — Predictive Latency Scoring (Backlog)

> Dynamic agent health metrics as a routing signal.

**Completed 2026-06-05:**

- [x] Build agent-pool / dispatch system (AgentRegistry, AgentDispatcher traits)
- [x] Define latency + error-rate scoring model (EWMA)
- [x] Collect per-call metrics: response time, failure rate, windowed averages (AgentMetricsCollector)
- [x] Solve cold-start problem (probationary default score for new agents)
- [x] Expose health status in API contract for clients
- [x] Circuit-breaker integration — skip chronically slow agents

---

## Phase 5 — Semantic Tool Versioning (Backlog)

> Multi-version contract support in `api-contract` crate.

**Deferred — premature. Existing serde defaults + `schema_version` field cover current needs.**

- [ ] Add warning log when unrecognized `schema_version` is received
- [ ] Defer multi-version parsing until backward-compatibility pain is confirmed
- [ ] If revisited: define version negotiation handshake (client advertises range, server responds)
- [ ] If revisited: define deprecation window and migration policy per version
