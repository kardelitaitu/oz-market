# HTTP Benchmark Baseline (2026-05-12)

## Scope

- Tool: `backend/target/release/bench_concurrent.exe`
- Target: `http://127.0.0.1:3000`
- Requests per level: `5000` (except fixed-mode diagnostic run at `2000`)
- Concurrency levels: `100,200,500`
- Server: `backend/target/release/oz-market-server.exe`
- DB: local Postgres (`compose.postgres.yml`)

## Results

| Mode | Search 100 | Search 200 | Search 500 | 429 Rate (search) | Notes |
|---|---:|---:|---:|---:|---|
| `public` | 57,733 ops/s | 57,350 ops/s | 51,569 ops/s | 0% | No claims header |
| `rotating` | 57,418 ops/s | 59,140 ops/s | 47,946 ops/s | 0% | Claims with rotating `sub` |
| `fixed` (diagnostic, 2k req) | 1,765 ops/s | 0 ops/s | 0 ops/s | 97-100% | Single `sub` hits 60/min limiter |

## Artifacts

- `docs/testing/benchmarks/http-bench-concurrent-public-2026-05-12.txt`
- `docs/testing/benchmarks/http-bench-concurrent-rotating-2026-05-12.txt`
- `docs/testing/benchmarks/http-bench-concurrent-fixed-2026-05-12.txt`

## Command examples

```powershell
backend/target/release/bench_concurrent.exe "http://127.0.0.1:3000" 5000 "100,200,500" "public"
backend/target/release/bench_concurrent.exe "http://127.0.0.1:3000" 5000 "100,200,500" "rotating"
backend/target/release/bench_concurrent.exe "http://127.0.0.1:3000" 2000 "100,200,500" "fixed"
```
