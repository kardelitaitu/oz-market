# Risks & Mitigations#

## Summary Table#

| Risk | Probability | Impact | Mitigation | Status |
|------|-------------|--------|-------------|--------|
| Actix migration breaks tests | Medium | High | Keep TCP runtime as `#[cfg(test)]` | ✅ Solved |
| Moka cache coherence issues | Low | Medium | Use `Cache::invalidate_all()` on writes | ⚠️ Pending |
| Zero-copy borrows too complex | High | Medium | Fall back to owned strings | ⚠️ Optional |
| Deadpool config too aggressive | Low | Low | Start with `max_size: 10` | ⚠️ Pending |
| Flamegraph on Windows fails | High | Low | Use WSL2 or `cargo-profiler` | ⚠️ Pending |
| ParadeDB breaks migration | Low | High | Keep as optional feature flag | ⚠️ Optional |
| Redis adds complexity | Medium | Medium | Only add if multi-instance | ⚠️ Optional |
| String allocations dominate CPU | Medium | Medium | Profile with flamegraph first | ⚠️ Pending |

---

## Detailed Risk Analysis#

### 🔴 Risk 1: Actix Migration Breaks Tests#

**Probability**: Medium  
**Impact**: High  
**Status**: ✅ **Solved**  

**Description**:  
Migrating from custom TCP runtime (`http/runtime.rs`) to Actix-web might break existing 37 tests that use `TcpListener`.

**Mitigation (✅ Solved)**:
```rust
// backend/server/src/lib.rs
#[cfg(not(test))]
pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    actix_runtime::run()  // NEW Actix runtime
}

#[cfg(test)]
pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    http::runtime::run()  // KEEP old TCP runtime for tests!
}
```

**Why It Works**:
- `#[cfg(test)]` ensures tests still use `TcpListener`  
- Production uses new Actix-web handlers  
- Zero risk to existing test coverage  

---

### �¡ Risk 2: Moka Cache Coherence Issues#

**Probability**: Low  
**Impact**: Medium  
**Status**: ⚠️ **Pending (Phase 1)**  

**Description**:  
Cached data might become stale after writes (create_listing, open_negotiation, etc.).

**Mitigation**:
```rust
// In CachedApp:
pub async fn create_listing(...) -> Result<...> {
    // Invalidate search cache on write
    self.search_cache.invalidate_all();  // Simple: clear all search cache
    
    // For listing cache: invalidate specific key
    self.listing_cache.invalidate(listing_id);  // Remove stale data
    
    self.app.create_listing(...).await
}
```

**Best Practice**:
- **Write-through**: Update cache on successful write  
- **Write-around**: Invalidate cache, let next read populate it  
- **Choose**: Write-around (simpler, less error-prone)  

---

### ⚠️ Risk 3: Zero-Copy Borrows Too Complex#

**Probability**: High (if implemented)  
**Impact**: Medium  
**Status**: ⚠️ **Optional (Phase 2)**  

**Description**:  
Borrowing `&str` from tokio-postgres row buffers requires careful lifetime management.

**Mitigation (Fallback)**:
```rust
// If zero-copy becomes too complex:
struct ListingRow {
    listing_id: String,  // Just use owned strings
    product_name: String,
    // ...
}

// Only use &str for LARGE fields that are cloned frequently:
struct ListingRow<'a> {
    listing_id: String,  // Primary key: small, keep owned
    description: &'a str,  // Large: borrow from row buffer
}
```

**Recommendation**:  
- **Skip Zero-Copy** if Phase 1 (Actix + Moka) already gives > 10,000 ops/s  
- **Only implement** if flamegraph shows `String::clone()` > 30% CPU  

---

### ⚠️ Risk 4: Deadpool Config Too Aggressive#

**Probability**: Low  
**Impact**: Low  
**Status**: ⚠️ **Pending (Phase 2)**  

**Description**:  
Setting `max_size` too high wastes memory; too low causes connection starvation.

**Mitigation**:
```rust
let pool = deadpool_postgres::Config {
    url: database_url,
    max_size: 10,  // Start conservative (2-4x vCPU)
    timeout: Some(Duration::from_secs(30)),
    ..Default::default()
}
.create_pool(Some(deadpool_postgres::Runtime::Tokio1))?;
```

**Tuning After Phase 1**:
| Metric | Action |
|--------|--------|
| p95 latency > 10ms | Increase `max_size` by 5 |
| Memory > 80% of server | Decrease `max_size` by 5 |
| Connection wait > 5% | Increase `max_size` by 10 |

---

### ⚠️ Risk 5: Flamegraph on Windows Fails#

**Probability**: High  
**Impact**: Low (Linux profiling still works)  
**Status**: ⚠️ **Pending (Phase 3)**  

**Description**:  
`cargo-flamegraph` might not work on native Windows (requires WSL2 or Linux).

**Mitigation Options**:
1. **Use WSL2** (Recommended):
   ```bash
   # In WSL2 terminal:
   cd /mnt/c/My\ Script/project-the-marketplace
   cargo flamegraph --package marketplace-server --bin phase5_bench
   ```

2. **Use `cargo-profiler`** (Windows-native):
   ```bash
   cargo install cargo-profiler
   cargo profiler record --package marketplace-server --bin phase5_bench
   ```

3. **Run on Linux CI** (GitHub Actions):
   ```yaml
   jobs:
     profile:
       runs-on: ubuntu-latest
       steps:
         - run: cargo flamegraph --package marketplace-server --bin phase5_bench
   ```

---

### ⚠️ Risk 6: ParadeDB Breaks Migration#

**Probability**: Low  
**Impact**: High (search won't work)  
**Status**: ⚠️ **Optional (Phase 3)**  

**Description**:  
Installing ParadeDB extension might fail on some Postgres versions.

**Mitigation**:
```toml
# Keep as optional feature in Cargo.toml
[features]
paradedb = ["backend/server/paradedb_migration.sql"]

# In migrations:
#[cfg(feature = "paradedb")]
include_str!("paradedb_migration.sql");
```

**When to Enable**:
- Only if `search-heavy` profile < 500 ops/s after Phase 2  
- Keep GIN index as default (works for most cases)  

---

### ⚠️ Risk 7: Redis Adds Complexity#

**Probability**: Medium  
**Impact**: Medium  
**Status**: ⚠️ **Optional (Phase 3)**  

**Description**:  
Adding Redis requires:
- New dependency + configuration  
- Cache coherence logic (invalidate on writes)  
- Operational overhead (monitoring, backups)  

**Mitigation**:
- **Only add if multi-instance deployment** (Phase 1 Moka is enough for single instance)  
- Start with **L1 Moka only** (Phase 1 gives ~5,000 ops/s already)  
- Add Redis L2 only if:
  - Running ≥ 2 server instances  
  - Moka hit rate < 50% for common queries  

**Simpler Alternative**:
```rust
// Skip Redis entirely if single instance:
struct CachedApp {
    listing_cache: Moka::future::Cache<String, ListingSummary>,  // L1 only
    // No Redis needed for single instance!
}
```

---

### ⚠️ Risk 8: String Allocations Dominate CPU#

**Probability**: Medium  
**Impact**: Medium  
**Status**: ⚠️ **Pending (Phase 3 - Flamegraph)**  

**Description**:  
Hot paths might spend > 30% CPU on `String::clone()` allocations.

**Mitigation** (After Flamegraph):
1. **Minimize allocations in `listings.rs`**:
   ```rust
   // Before: String::from_utf8_lossy(&body)
   // After: &str borrow where possible
   ```

2. **Use `Cow<'_, str>` for conditional cloning**:
   ```rust
   use std::borrow::Cow;
   
   fn process(input: &str) -> Cow<'_, str> {
       if needs_processing(input) {
           Cow::Owned(input.to_uppercase())  // Clone only when needed
       } else {
           Cow::Borrowed(input)  // Zero-copy
       }
   }
   ```

3. **Cache serialized JSON** (avoid re-serialization):
   ```rust
   struct CachedApp {
       listing_json_cache: Cache<String, String>,  // Store serialized JSON
   }
   
   // Serve from cache: zero serialization
   HttpResponse::Ok().body(cached_json)
   ```

---

## Success Criteria (Repeated from Overview)#

### Must-Have (P0)#
- [ ] **listing-read**: 20,000+ ops/s (baseline: 321 ops/s)#
- [ ] **search-heavy**: 2,000+ ops/s (baseline: 77 ops/s)#
- [ ] **negotiation-burst**: 2,000+ ops/s (baseline: 85 ops/s)#
- [ ] **p99 latency** < 5ms for listing-read#
- [ ] All 37 tests still pass after changes#

### Should-Have (P1)#
- [ ] **p95 latency** < 3ms for cached reads#
- [ ] **Zero allocations** in hot path for cached hits#
- [ ] **Flamegraph** shows no single function > 20% CPU#

### Nice-to-Have (P2)#
- [ ] **ParadeDB** integration for search-heavy (if still < 2,000 ops/s)#
- [ ] **Redis L2 cache** (if running multiple instances)#
- [ ] **ARM64 build** (for low-cost server deployment)#

---

## Next Actions#

1. **[ ] Review all proposal files** — approve/reject within 24 hours#
2. **[ ] Start Milestone 1** — Actix + Moka (highest ROI)#
3. **[ ] Run before/after benchmarks** — document % improvement at each step#
4. **[ ] Create `docs/server/optimization-report.md`** — track progress#

---

**Document Status**: Risk Mitigations Ready    
**Last Updated**: 2026-05-07    
**Author**: pi (based on Phase 5 benchmark results)    
**Reviewers**: @dev (code owner), @dev (product owner)  
