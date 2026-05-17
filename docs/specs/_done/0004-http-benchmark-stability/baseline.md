# Baseline: HTTP Benchmark Stability and Reproducibility

## What I Find

### Current State

- Benchmark utility supports multiple claims modes.
- Root and server docs now include benchmark mode guidance.
- Dated artifacts exist under `docs/testing/benchmarks/`.

### Current Gap

- The process is partially codified but not yet managed as an active spec lifecycle item.
- Ongoing benchmark updates need explicit acceptance checks to avoid drift.

### Why This Matters

Benchmark regressions are easy to misinterpret when load identity and limiter behavior are not made explicit.

## What I Claim

The next active work should enforce benchmark reproducibility as a governed workflow, not ad hoc command usage.

## What Is the Proof

1. Fixed-identity runs can trigger limiter saturation and lower headline ops/s.
2. Rotating/public modes provide different throughput characteristics.
3. Without explicit mode tagging, cross-day comparisons are ambiguous.
