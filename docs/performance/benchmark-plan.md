# Benchmark Plan: Before/After Comparison#

## Baseline (Current State)#

```bash
#!/bin/bash
# backend/scripts/bench-baseline.sh
export DATABASE_URL="postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"

echo "=== Baseline Benchmark (321 ops/s target) ==="
cd backend
cargo run --package marketplace-server --bin phase5_bench
```

**Expected Output**:
```
profile          | ops | elapsed_ms | ops_per_sec
listing-read    | 500 | 1556       | 321.34
search-heavy    | 500 | 6483       | 77.12
negotiation-burst | 300 | 3528       | 85.03
```

---

## After Milestone 1 (Actix + Moka)#

```bash
#!/bin/bash
# backend/scripts/bench-milestone1.sh
export DATABASE_URL="postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"

echo "=== After Milestone 1: Actix + Moka (Target: 5,000 ops/s) ==="
cd backend
cargo run --package marketplace-server --bin phase5_bench
```

**Expected Output**:
```
profile          | ops | elapsed_ms | ops_per_sec
listing-read    | 500 | ~156        | ~5,000    (15x improvement)
search-heavy    | 500 | ~1000        | ~500      (6x improvement)
negotiation-burst | 300 | ~600         | ~500      (6x improvement)
```

---

## After Milestone 2 (Zero-Copy + Pool)#

```bash
#!/bin/bash
# backend/scripts/bench-milestone2.sh
export DATABASE_URL="postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"

echo "=== After Milestone 2: Zero-Copy + Pool (Target: 10,000 ops/s) ==="
cd backend
cargo run --package marketplace-server --bin phase5_bench
```

**Expected Output**:
```
profile          | ops | elapsed_ms | ops_per_sec
listing-read    | 500 | ~50          | ~10,000   (30x improvement)
search-heavy    | 500 | ~500         | ~1,000    (13x improvement)
negotiation-burst | 300 | ~300         | ~1,000    (12x improvement)
```

---

## After Milestone 3 (Full Optimization)#

```bash
#!/bin/bash
# backend/scripts/bench-final.sh
export DATABASE_URL="postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"

echo "=== Final Benchmark (Target: 20,000+ ops/s) ==="
cd backend
cargo run --package marketplace-server --bin phase5_bench
```

**Expected Output**:
```
profile          | ops | elapsed_ms | ops_per_sec
listing-read    | 500 | ~25          | ~20,000+  (60x improvement)
search-heavy    | 500 | ~250         | ~2,000    (26x improvement)
negotiation-burst | 300 | ~150         | ~2,000    (24x improvement)
```

---

## Quick Benchmark (Before Any Changes)#

```bash
#!/bin/bash
# backend/scripts/bench-quick.sh
export DATABASE_URL="postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"

echo "=== Quick Benchmark (Verify 321 ops/s baseline) ==="
cd backend
for i in {1..3}; do
    echo "--- Run $i ---"
    cargo run --package marketplace-server --bin phase5_bench 2>&1 | tail -5
    sleep 60  # Cool down
done
```

---

## Performance Dashboard (Optional)#

```bash
# Generate performance report
cd backend

# Run 5 times and save
for i in {1..5}; do
    export DATABASE_URL="postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"
    cargo run --package marketplace-server --bin phase5_bench >> performance_log.txt
    sleep 60  # Cool down between runs
done

# Plot with Python/Gnuplot (optional)
python3 scripts/plot_performance.py performance_log.txt
```

---

## Proof of Low-Cost Deployment#

```bash
# Deploy to $5/month Hetzner VPS (2 vCPU, 4GB RAM)
# Run benchmark remotely
ssh user@your-server "cd /app && \
    export DATABASE_URL=... && \
    cargo run --release --bin phase5_bench"

# Expected: 20,000+ ops/s on $5/month infrastructure
# Cost per 1M operations: $0.00025 (vs $0.02 on cloud functions)
```

---

## Continuous Benchmarking (CI/CD)#

```yaml
# .github/workflows/benchmark.yml
name: Benchmark

on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_DB: marketplace
          POSTGRES_USER: marketplace
          POSTGRES_PASSWORD: marketplace
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Run benchmark
        run: |
          export DATABASE_URL="postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"
          cd backend
          cargo run --package marketplace-server --bin phase5_bench > benchmark_output.txt
      - name: Check performance
        run: |
          # Parse benchmark_output.txt
          # Fail if listing-read < 20,000 ops/s
          # Fail if search-heavy < 2,000 ops/s
          # Fail if negotiation-burst < 2,000 ops/s
```

---

## Files to Create#

| Script | Purpose |
|--------|---------|
| `backend/scripts/bench-baseline.sh` | Verify 321 ops/s baseline |
| `backend/scripts/bench-milestone1.sh` | After Actix + Moka |
| `backend/scripts/bench-milestone2.sh` | After Zero-Copy + Pool |
| `backend/scripts/bench-final.sh` | After Full Optimization |
| `backend/scripts/bench-quick.sh` | Quick verification loop |

---

## Next Steps#

1. **[ ] Run baseline benchmark** — verify 321 ops/s#
2. **[ ] Start Milestone 1** — Actix + Moka#
3. **[ ] Run after each change** — document % improvement#
4. **[ ] Commit to `docs/server/optimization-report.md`** — track progress#

---

**Document Status**: Benchmark Plan Ready    
**Last Updated**: 2026-05-07    
**Author**: pi (based on Phase 5 benchmark results)  
