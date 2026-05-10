# Baseline

## What I Find
Current infrastructure configuration shows critical limitations for high-concurrency workloads:
- Database connection pool limited to 20 connections (backend/server/src/http/actix_runtime.rs:265)
- Default tokio runtime configuration without optimization for high concurrency
- Basic Moka cache with small sizes (1k for search, 10k for listings) and no TTL policies
- No connection pool monitoring or dynamic scaling

## What I Claim
By expanding database connection pool to 200+ connections, optimizing tokio runtime with 64+ worker threads, and implementing sophisticated caching with 50k+ capacity and TTL, throughput will recover to 40k+ ops/s at 5000 concurrency with P95 latency under 50ms.

## What Is the Proof
- Benchmark results show 47k ops/s at 100 concurrency degrading to 6.8k ops/s at 5000 concurrency
- Database connection pool size of 20 is insufficient for 5000 concurrent requests (likely causing queueing)
- Cache sizes (1k search, 10k listings) are too small for diverse query patterns at scale
- Actix server uses default worker count (CPU cores) but tokio runtime defaults may not support optimal thread utilization