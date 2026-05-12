# Plan: HTTP Benchmark Stability and Reproducibility

## What Is the Solution

### Step 1: Canonical Commands

1. Lock benchmark command profiles for `public`, `rotating`, and `fixed` claims modes.
2. Keep parameters explicit in docs (`threads`, `concurrency`, `requests`, `warmup`).
3. Require date-stamped artifact output filenames.

### Step 2: Comparable Metrics

1. Keep ops/s as the primary metric.
2. Keep `429` and `other_failures` as first-class counters.
3. Require mode label in each summary row.

### Step 3: Parity and Validation

1. Recheck status-code parity across runtime and handler paths used by benchmark flows.
2. Run full `./check.ps1` before committing benchmark-affecting changes.
3. Record benchmark outcome and interpretation in `JOURNAL.md`.

### Step 4: Baseline Discipline

1. Keep one dated baseline report per run cycle.
2. Include run date and route profile assumptions in report header.
3. Note if benchmark is saturation diagnostic (`fixed`) versus throughput baseline (`public` or `rotating`).

## Success Metrics

- repeated runs of the same mode are directionally stable
- throughput comparisons include explicit mode context
- rate-limit effects are visible, not hidden in aggregate ops/s
- benchmark claims in docs are traceable to dated artifacts

## Phased Rollout Plan

1. Confirm documentation and report templates.
2. Run benchmark profiles and refresh artifacts.
3. Validate with checker and journal checkpoint.
4. Move spec to `_done` after acceptance criteria are confirmed.
