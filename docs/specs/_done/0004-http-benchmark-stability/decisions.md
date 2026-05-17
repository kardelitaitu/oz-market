# Decisions: HTTP Benchmark Stability and Reproducibility

## Decision 1: Canonical Baseline Mode

Use `rotating` claims mode as the default throughput baseline for authenticated-path stress tests.

## Decision 2: Keep `fixed` as Diagnostic

Keep `fixed` claims mode only as a limiter-saturation diagnostic, not as the headline throughput baseline.

## Decision 3: Artifact Discipline

Require date-stamped output and one parity/baseline note per run cycle.

## Alternatives

| Option | Pros | Cons |
|---|---|---|
| `rotating` as baseline (chosen) | closer to realistic identity mix, less artificial limiter saturation | slightly more run variance than single-identity tests |
| `fixed` as baseline | highly deterministic identity | distorts throughput via predictable 429 saturation |
| `public` as baseline | simple and stable for unauthenticated profile | not representative for authenticated-agent workloads |
