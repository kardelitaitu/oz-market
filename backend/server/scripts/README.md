# Server Scripts

## Phase 5 Benchmark Runner

Use `run-phase5-bench.ps1` to measure search and write behavior against Postgres-backed storage.

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

To run the Postgres integration tests against the same local database:

```powershell
.\backend\server\scripts\run-postgres-tests-local.ps1
```

To run both steps in sequence:

```powershell
.\backend\server\scripts\run-local-postgres-dev.ps1
```
