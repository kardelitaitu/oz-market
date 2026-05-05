# Server Scripts

## Phase 5 Benchmark Runner

Use `run-phase5-bench.ps1` to measure search and write behavior against Postgres-backed storage.

### Required Input

- `DATABASE_URL` pointing at a live Postgres instance

### Run

```powershell
.\backend\server\scripts\run-phase5-bench.ps1 -DatabaseUrl $env:DATABASE_URL
```

### Notes

- The in-memory fallback is for smoke checks only.
- Use the Postgres run before changing quota or index settings.

## Local Postgres Path

Start the local database with:

```powershell
docker compose -p marketplace -f compose.postgres.yml up -d
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
