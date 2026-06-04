---
id: 0017-agent-circuit-breaker-health-api
title: Agent Circuit-Breaker and Health API
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Agent Circuit-Breaker and Health API

Status: `active`
Implementer: `agent`

## Summary

This specification defines the circuit-breaker state machine and API endpoints for exposing agent health status. Slow or chronically failing agents are temporarily bypassed by the router, while clients can query the system to view performance telemetry.

## Scope

### In Scope
- Designing the circuit-breaker state transitions: `Closed`, `Open`, `Half-Open`.
- Intercepting dispatches to bypass unhealthy/circuit-broken agents.
- Implementing REST endpoints to list and retrieve agent health status details.
- Specifying the OpenAPI contract updates for the health endpoints.

### Out of Scope
- Custom dashboard UIs for monitoring agent health (deferred to frontend team).

## Proposed Direction
1. Circuit Breaker Logic:
   - Tracks status per-agent in-memory.
   - Trips to `Open` after 5 consecutive failures (i.e. failure count reaches 5 without an intervening success), or if the most recent response exceeded 2000ms in latency (slow responses are counted as failures).
   - Stays `Open` for 30 seconds before transitioning to `Half-Open` for testing.
   - Note: a single successful response resets the consecutive-failure counter; the breaker only trips from a sustained failure streak. The original 20%-error-rate threshold was revised to "5 consecutive failures" during implementation (see `decisions.md` for the rationale) to give operators a simpler mental model and to avoid edge cases at low sample counts where a single failure can dominate the percentage.
2. API Endpoints:
   - `GET /v1/health/agents`: Returns summary status of all agents.
   - `GET /v1/health/agents/{id}`: Detailed status with EWMA values.
