# CLI README

Common commands for day-to-day development and testing.

## From project root

### Verify code

```powershell
.\check.ps1
```

### Format code

```powershell
cd backend
cargo fmt --all
```

### Lint

```powershell
cd backend
cargo clippy --workspace
```

### Run tests

```powershell
cd backend
cargo test --lib
```

## Backend development

### Start the HTTP server

```powershell
cd backend
cargo run --package marketplace-server
```

### Seed the database once

```powershell
cargo run --manifest-path backend/Cargo.toml -p marketplace-server --bin populate_db
```

### Run the real HTTP benchmark

```powershell
.\backend\scripts\bench-http.ps1 -Ops 1000 -ConcurrencyLevels "1,10,50,100,250,500,1000" -SeedDatabase
```

### Run the local Postgres dev flow

```powershell
.\backend\server\scripts\run-local-postgres-dev.ps1
```

## Backend database checks

### Benchmark against local Postgres

```powershell
.\backend\server\scripts\run-phase5-bench-local.ps1
```

### Run Postgres integration tests

```powershell
.\backend\server\scripts\run-postgres-tests-local.ps1
```

## Notes

- Use `check.ps1` before commits.
- Prefer the HTTP benchmark when measuring real server performance.
- Seed once, benchmark many.
- The real benchmark is the Actix server + HTTP path, not the direct app/repo benchmark.
- Use lower concurrency first, then sweep higher levels to see saturation.
- Current baseline for future comparisons: release build with cache on, search peaks around 6.5k ops/s and get_listing around 4.6k ops/s.
