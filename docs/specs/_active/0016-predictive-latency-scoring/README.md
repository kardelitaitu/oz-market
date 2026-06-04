---
id: 0016-predictive-latency-scoring
title: Predictive Latency Scoring
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Predictive Latency Scoring

Status: `active`
Implementer: `agent`

## Summary

This specification describes the algorithm and implementation of the dynamic predictive latency scoring engine. It calculates Exponentially Weighted Moving Averages (EWMA) for agent latency and error rates to provide dynamic routing signals.

## Scope

### In Scope
- Implementing the EWMA computation model for latency and error rates.
- Handling the probationary cold-start score configuration for newly registered agents.
- Exposing a combined score value for routing selection.

### Out of Scope
- Actually storing individual sample measurements (managed by Spec 0015).
- Exposing agent statuses via REST endpoint or circuit breaker (managed by Spec 0017).

## Proposed Direction
1. EWMA Model:
   - For latency: $\text{EWMA}_L = \alpha \cdot L_{\text{new}} + (1 - \alpha) \cdot \text{EWMA}_{L,\text{old}}$
   - For error rate: $\text{EWMA}_E = \alpha \cdot E_{\text{new}} + (1 - \alpha) \cdot \text{EWMA}_{E,\text{old}}$
   - Configurable smoothing factor $\alpha$ (default: 0.2).
2. Probationary Period:
   - New agents are assigned a default probationary status.
   - Their initial scores are configured to a default baseline (e.g. 200ms latency, 0% error rate).
