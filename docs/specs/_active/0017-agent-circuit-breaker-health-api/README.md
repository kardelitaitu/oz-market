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
   - Trips to `Open` if error rate exceeds 20% or latency exceeds 2000ms.
   - Stays `Open` for 30 seconds before transitioning to `Half-Open` for testing.
2. API Endpoints:
   - `GET /v1/health/agents`: Returns summary status of all agents.
   - `GET /v1/health/agents/{id}`: Detailed status with EWMA values.
