# HTTP Benchmark Cycle (2026-05-12)

## Scope

- Tool: `backend/target/release/bench_concurrent.exe`
- Target: `http://127.0.0.1:3000`
- Requests per level: `5000` for `public` and `rotating`, `2000` for `fixed`
- Concurrency levels: `100,200,500`
- Server: `backend/target/release/marketplace-server.exe`
- DB: local Postgres (`compose.postgres.yml`)

## Results

| Mode | Search 100 | Search 200 | Search 500 | 429 Rate (search) | Notes |
|---|---:|---:|---:|---:|---|
| `public` | 58,684.21 ops/s | 59,509.36 ops/s | 55,881.28 ops/s | 0% | No claims header |
| `rotating` | 58,676.08 ops/s | 60,841.39 ops/s | 52,905.29 ops/s | 0% | Claims with rotating `sub` |
| `fixed` (diagnostic, 2k req) | 4,488.23 ops/s | 4,604.07 ops/s | 4,410.62 ops/s | 0% | Single `sub`; limiter did not saturate in this run |

## Notes

- Health and warm `get_listing` were also exercised during the cycle.
- This cycle is materially different from the dated baseline because `fixed` mode no longer showed the old 429 saturation pattern.
- Keep the baseline report and this cycle report together when comparing throughput across days.
