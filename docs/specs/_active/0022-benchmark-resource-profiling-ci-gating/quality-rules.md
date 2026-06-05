# Quality Rules - Benchmark Resource Profiling and CI Gating

- **Minimal Profiling Footprint**: Keep the refresh rate slow (500ms) to ensure the system metrics gatherer does not perturb actual benchmark runs.
- **Accurate Resource Bounds**: Threshold violations must fail the run immediately, and yield clear stdout logs detailing which parameter caused the build failure.
- **Deterministic Exit Handling**: All failure outcomes (timeouts, errors, threshold violations) must result in a clean non-zero terminal exit code (`1`).
