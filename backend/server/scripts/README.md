# Server Scripts

## Phase 5 Benchmark Runner

Use `run-phase5-bench.ps1` to measure search and write behavior against Postgres-backed storage.
This is the low-level app/repo benchmark.

### Required Input

- `DATABASE_URL` pointing at a live Postgres instance
- a database already seeded with the current schema/data generator

### Run

```powershell
.\backend\server\scripts\run-phase5-bench.ps1 -DatabaseUrl $env:DATABASE_URL
```

### Notes

- The benchmark runner now assumes the schema is already migrated.
- Seed the database once with `populate_db` and reuse it for repeated runs.
- The in-memory fallback is for smoke checks only.
- Use the Postgres run before changing quota or index settings.
- For the real Actix/HTTP benchmark, use `backend/scripts/bench-http.ps1` (release build, concurrency sweep, percentiles).
- Future benchmark baseline: release build with cache on, search around 6.5k ops/s and get_listing around 4.6k ops/s.

## Local Postgres Path

Start the local database with:

```powershell
docker compose -p marketplace -f compose.postgres.yml up -d
```

Seed once with the current generator:

```powershell
cargo run --manifest-path backend/Cargo.toml -p marketplace-server --bin populate_db
```

Then run:

```powershell
.\backend\server\scripts\run-phase5-bench-local.ps1
```

- The local benchmark runner bootstraps the schema through `bootstrap_schema` before invoking `phase5_bench`.

To run the Postgres integration tests against the same local database:

```powershell
.\backend\server\scripts\run-postgres-tests-local.ps1
```

- The test runner bootstraps the schema through `bootstrap_schema` before invoking the integration tests.

To run both steps in sequence:

```powershell
.\backend\server\scripts\run-local-postgres-dev.ps1
```

- The combined workflow bootstraps the schema once, then skips duplicate setup in the child scripts.
