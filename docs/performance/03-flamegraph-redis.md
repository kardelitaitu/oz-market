# Phase 3: Redis L2 + Flamegraph (Milestone 3)#

## Goal: ~20,000+ ops/s on listing-read (60x improvement)#

## Flamegraph-Driven Optimization#

### Step 3.1: Install & Run#

```bash
# Install flamegraph
cargo install flamegraph

# On Windows: use WSL2 or cargo-profiler
# Run benchmark with profiling
cd backend
cargo flamegraph --package oz-market-server --bin phase5_bench \
    --bench-profile listing-read \
    --output flamegraph.svg

# Open flamegraph.svg in browser
```

### Step 3.2: Analyze Hotspots#

Typical findings:
1. **String allocations** in `listings.rs` → use `&str` borrows  
2. **JSON serialization** in `SearchResponse` → cache serialized form  
3. **Lock contention** in `RwLock` → use `DashMap` or sharded locks  

### Step 3.3: Optimize Top 3 Hotspots#

```rust
// Example: Minimize allocations in hot path
// Before: String::from_utf8_lossy(&body)
// After: Use &str borrows where possible

// Example: Cache serialized JSON
use moka::future::Cache;

struct CachedApp {
    // Cache serialized form to avoid re-serialization
    listing_json_cache: Cache<String, String>,  // key: listing_id, value: serialized JSON
}
```

---

## Redis L2 Cache (Optional)#

### When to Add Redis#

- **Only if running multiple server instances**  
- Single instance: Moka L1 is sufficient (Phase 1 gives ~5,000 ops/s)  
- Multi-instance: Add Redis for shared cache coherence  

### Redis Architecture#

```
Request → Actix-web → Moka (L1) → Redis (L2) → Postgres
```

### Redis Implementation (Optional)#

```rust
// backend/server/Cargo.toml
[dependencies]
redis = { version = "0.23", features = ["tokio"] }
```

```rust
// backend/server/src/http/cached_app.rs
use redis::AsyncCommands;

// Note: For Redis L2, add to CachedApp:
// pub struct CachedApp<LR, IR, RR, CR> {
//     ...,
//     redis_pool: redis::aio::ConnectionManager, // or a pool type
// }

impl CachedApp {
    async fn get_listing_with_redis(
        &self, 
        claims: &Claims, 
        listing_id: &str
    ) -> Result<Option<ListingSummary>, HandlerError> {
        let cache_key = listing_id.to_string();
        
        // L1: Moka (in-process)
        if let Some(cached) = self.listing_cache.get(&cache_key) {
            return Ok(Some(cached));
        }
        
        // L2: Redis (shared)
        let mut conn = self.redis_pool.get().await?;
        if let Ok(Some(cached)) = redis::cmd("GET")
            .arg(&cache_key)
            .query_async(&mut conn)
            .await 
        {
            let listing: ListingSummary = serde_json::from_str(&cached)?;
            self.listing_cache.insert(cache_key, listing.clone());
            return Ok(Some(listing));
        }
        
        // L3: Postgres
        let result = self.app.get_listing(claims, listing_id).await?;
        if let Some(ref listing) = result {
            // Store in Redis L2
            let _ = redis::cmd("SET")
                .arg(&cache_key)
                .arg(serde_json::to_string(listing)?)
                .arg("EX")  // TTL
                .arg(300)   // 5 minutes
                .query_async(&mut conn)
                .await;
        }
        Ok(result)
    }
}
```

**Expected Impact**: 2x improvement over Phase 2 (~10,000 → ~20,000+ ops/s)  

---

## ParadeDB (Optional, for search-heavy)#

### When to Add ParadeDB#

- **Only if `search-heavy` profile < 500 ops/s target**  
- Postgres extension for real-time search  
- Replaces `search_text` GIN index with ParadeBM index  

### ParadeDB Integration#

```sql
-- Only if needed!
CREATE EXTENSION IF NOT EXISTS paradedb;

-- Replace GIN index
DROP INDEX IF EXISTS idx_listings_search_text;
CREATE INDEX idx_listings_search_text 
    ON listings USING bm25 (search_text paradedb.bm25_update);
```

**Expected Impact**: 2-3x improvement for search profile (77 → ~200 ops/s)  

---

## Files to Modify#

| File | Action | Purpose |
|------|--------|---------|
| `backend/server/Cargo.toml` | **Modify** | Add redis dependency (optional) |
| `backend/server/src/http/cached_app.rs` | **Modify** | Add Redis L2 fallback |
| `backend/server/src/repositories/listings.rs` | **Modify** | Optimize hotspots from flamegraph |
| `backend/server/migrations/` | **Modify** | Add ParadeDB if needed |

---

## Expected Impact (Full Optimization)#

| Profile | Baseline | After Phase 1 | After Phase 2 | After Phase 3 |
|---------|---------|---------------|---------------|---------------|
| listing-read | 321 | ~5,000 | ~10,000 | ~20,000+ |
| search-heavy | 77 | ~500 | ~1,000 | ~2,000 |
| negotiation-burst | 85 | ~500 | ~1,000 | ~2,000 |

---

## Risks & Mitigations#

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-------------|
| Flamegraph on Windows fails | High | Low | Use WSL2 or `cargo-profiler` |
| Redis adds complexity | Medium | Medium | Only add if multi-instance |
| ParadeDB breaks migration | Low | High | Keep as optional feature flag |
| Zero-copy borrows too complex | High | Medium | Fall back to owned strings |

---

## Next Steps#

1. **[ ] Complete Phase 1 FIRST** — Actix + Moka (highest ROI)  
2. **[ ] Run flamegraph** on Phase 1 results  
3. **[ ] Optimize top 3 hotspots** — document % improvement  
4. **[ ] Add Redis L2 if needed** — only if multi-instance   
5. **[ ] Final benchmark** — expect ~20,000+ ops/s  

---

**Document Status**: Phase 3 Details (Optional)    
**Last Updated**: 2026-05-07    
**Author**: pi (based on Phase 5 benchmark results)  
**Dependencies**: Phase 1 & 2 MUST be complete first  
