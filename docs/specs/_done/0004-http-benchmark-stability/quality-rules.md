# Quality Rules

1. Always state claims mode in benchmark commands and reports.
2. Always include `429` and `other_failures` in summary output.
3. Never compare runs across different modes without calling out the mode change.
4. Keep dated artifacts under `docs/testing/benchmarks/`.
5. Run `cargo check` before commit for benchmark-related backend changes.
