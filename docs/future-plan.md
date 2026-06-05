# Future Optimization & Upgrade Plan

> **Purpose**: Track optional optimizations and future upgrades that are NOT immediately required.
> 
> **Current Status (2026-05-07)**: Phase 1 complete (7,281 ops/s). Phase 2 skipped due to low ROI.

---

## Table of Contents

1. [Current Achievement](#current-achievement)
2. [Phase 2: Zero-Copy + Pool (OPTIONAL)](#phase-2-zero-copy--pool-optional)
3. [Phase 3: Redis L2 Cache (OPTIONAL)](#phase-3-redis-l2-cache-optional)
4. [Other Future Improvements](#other-future-improvements)
5. [Decision Matrix](#decision-matrix)

---

## Current Achievement

### Phase 1: Actix-Web + Moka Cache ✅

**Completed**: 2026-05-07  
**Result**: **7,281 ops/s** (22.7× improvement over 321 ops/s baseline)  
**Target**: 5,000 ops/s ✅ (exceeded by 45%)

**What was done:**
- Migrated from custom TCP runtime to Actix-Web 4
- Added Moka 0.12 cache (JSON string caching)
- Release build optimization
- Auth via `x-marketplace-claims` header

**Status**: Production-ready. No further optimization required unless scaling to multiple instances.

---

## Phase 2: Zero-Copy + Pool (OPTIONAL)

### Overview

**Original Goal**: ~10,000 ops/s (30× improvement)  
**Revised Assessment**: Low ROI after Phase 1 success  

### Why It Was Skipped (2026-05-07)

1. **Phase 1 already exceeded target**: 7,281 ops/s (vs 5,000 target)
2. **Poor ROI**: ~2× max improvement (7,281 → ~14,000 ops/s)
3. **High complexity**: 
   - `tokio-postgres` migration from `sqlx`
   - Zero-copy borrowing requires major refactoring
   - `deadpool-postgres` API compatibility issues (v0.10 has different Config struct)
4. **Existing pooling**: `sqlx::PgPool` with `max_connections(20)` already provides good connection pooling

### What Would Be Involved (If Ever Implemented)

#### 2.1: deadpool-postgres Connection Pool

**Dependencies:**
```toml
deadpool = "0.10"
deadpool-postgres = "0.10"
tokio-postgres = "0.7"
```

**Purpose**: Better tail latency (p95/p99) than sqlx pool  
**Expected Gain**: Minimal on average ops/s, better tail latency only

#### 2.2: Zero-Copy Row Mapping

**Concept**: Borrow `&str` from tokio-postgres row buffers instead of cloning Strings

**Example (theoretical):**
```rust
// Instead of sqlx::FromRow (which clones)
impl<'a> FromSql<'a> for ZeroCopyListing {
    fn from_sql(row: &'a Row) -> Result<Self, Box<dyn Error>> {
        Ok(ZeroCopyListing {
            listing_id: row.get(0),  // owned (small)
            product_name: row.get(1),  // owned (small)
            description: row.get::<_, &str>(2)?,  // BORROWED from row buffer!
        })
    }
}
```

**Challenges:**
- `ListingSummary` struct requires owned `String`s (API contract)
- `serde::Serialize` requires owned data for JSON responses
- Would need major refactoring of `marketplace_api_contract` types
- Debuggability issues with borrowed references

#### 2.3: When to Reconsider Phase 2

| Condition | Action |
|-----------|--------|
| Phase 1 reaches plateau at scale | Re-evaluate |
| Flamegraph shows allocation > 30% CPU | Implement for hot paths ONLY |
| Multi-instance deployment needed | **Do Phase 3 instead** |
| Tail latency (p99) > 10ms in production | Tune sqlx pool first |

### Recommendation

**SKIP Phase 2** unless:
- You have concrete flamegraph evidence of allocation bottlenecks
- You need > 10,000 ops/s (unlikely for single instance)
- You're deploying multiple instances (then do Phase 3)

**Better use of time:**
- Production hardening (telemetry, error handling)
- Phase 3 (Redis L2) if multi-instance needed
- New features / business logic

---

## Phase 3: Redis L2 Cache (OPTIONAL)

### Goal

Add Redis as L2 cache for multi-instance deployment.

### Why Consider It?

- Phase 1 Moka cache is **per-instance** (L1, in-memory)
- Redis would be **shared L2** across multiple Actix server instances
- Enables horizontal scaling

### Implementation Sketch

```rust
// Pseudo-code for L1 (Moka) + L2 (Redis) architecture
pub struct L1L2Cache<K, V> {
    l1: Cache<K, V>,  // Moka per-instance
    l2: RedisClient,  // Shared Redis
}

impl<K, V> L1L2Cache<K, V> {
    pub async fn get(&self, key: &K) -> Option<V> {
        // L1 first
        if let Some(v) = self.l1.get(key).await {
            return Some(v);
        }
        // L2 second
        if let Some(json) = self.l2.get(&serialize(key)?).await? {
            let value: V = serde_json::from_slice(&json)?;
            self.l1.insert(key.clone(), value.clone()).await;
            return Some(value);
        }
        None
    }
}
```

### Dependencies

```toml
redis = { version = "0.27", features = ["tokio-comp"] }
```

### Expected Impact

- **Single instance**: No improvement (adds latency to check Redis)
- **Multi-instance**: Consistent cache across instances
- **Cache coherence**: Need invalidation strategy on writes

### When to Implement

- [ ] Deploying multiple Actix server instances
- [ ] Need shared cache state across instances
- [ ] Phase 1 stable in production

---

## Other Future Improvements

### A. Production Hardening (Recommended Next)

| Area | Priority | Effort |
|------|----------|--------|
| Structured logging (tracing crate) | High | Low |
| Metrics (prometheus/prometheus-client) | High | Medium |
| Health checks (deep checks: DB, Redis) | Medium | Low |
| Graceful shutdown | Medium | Low |
| CI/CD pipeline (GitHub Actions) | High | Medium |
| Error handling improvements | Medium | Low |

### B. Mobile Client (Phase 4)

- Android scaffold: `mobile/app-android/`
- iOS scaffold: `mobile/app-ios/`
- Shared contract via `oz-market-api-contract`

### C. Advanced Features

- [ ] GraphQL API alternative
- [ ] WebSocket for real-time updates
- [ ] Image upload for listings
- [ ] Full-text search optimization (PostgreSQL tsvector)
- [ ] Rate limiting (actix-governor)
- [ ] API versioning strategy

---

## Decision Matrix

| Upgrade | Status | ROI | Complexity | When to Do It |
|---------|--------|-----|------------|----------------|
| **Phase 1: Actix + Moka** | ✅ Done | High (22.7×) | Medium | ✅ Complete |
| **Phase 2: Zero-Copy** | ❌ Skipped | Low (~2×) | High | If flamegraph shows bottlenecks |
| **Phase 3: Redis L2** | 📋 Optional | Medium | Medium | Multi-instance deployment |
| **Production Hardening** | 📋 Recommended | High | Low-Medium | Next |
| **Mobile Client** | 📋 Optional | Business decision | High | When ready |

---

## How to Re-enable Phase 2 (If Needed Later)

### Step 1: Check Flamegraph

```bash
# Install flamegraph
cargo install flamegraph

# Profile the server
cargo flamegraph --package oz-market-server --bin oz-market-server

# Check if allocation is > 30% CPU
# If yes, proceed to Step 2
```

### Step 2: Fix API Compatibility

The earlier attempt failed due to `deadpool-postgres` v0.10 API differences:

**Problem**: Config struct has no `url` or `pool_max_size` fields  
**Solution**: Use the correct API:

```rust
use deadpool_postgres::{Config, Manager, Runtime};

let mut cfg = Config::new();
cfg.dbname = Some("marketplace".to_string());
cfg.host = Some("localhost".to_string());
cfg.port = Some(5432);
cfg.user = Some("marketplace".to_string());
cfg.password = Some("marketplace".to_string());
cfg.pool = Some(deadpool::managed::PoolConfig {
    max_size: 20,
    ..Default::default()
});

let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
```

### Step 3: Gradual Migration

1. Start with `deadpool` for **read-only** hot paths (`get_listing`)
2. Keep `sqlx` for writes and complex queries
3. Add feature flag: `#[cfg(feature = "zero-copy")]`
4. Benchmark after each change

---

**Document Status**: Living document (update as decisions change)  
**Last Updated**: 2026-05-07  
**Author**: pi  
**Next Review**: When scaling beyond single instance
