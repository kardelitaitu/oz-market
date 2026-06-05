# Validation Checklist - Delete Legacy Benchmarks

This checklist is used to confirm the completion of Spec 0023:

- [ ] Legacy benchmark files in `backend/server/src/bin/` are deleted.
- [ ] Criterion target `backend/server/benches/search_bench.rs` is deleted.
- [ ] Legacy powershell scripts (`bench-http.ps1`, `run-phase5-bench-local.ps1`, `run-phase5-bench.ps1`) are deleted.
- [ ] `backend/server/Cargo.toml` no longer contains target mappings for deleted benchmarks.
- [ ] Backend compiles successfully with `cargo check` after deletions.
