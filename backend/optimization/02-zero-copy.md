# Phase 2: Zero-Copy + Connection Pooling (Milestone 2)#

## Goal: ~10,000 ops/s on listing-read (30x improvement)#

## Why Zero-Copy?#

Current sqlx uses runtime-determined row copying. Direct tokio-postgres with careful buffer management reduces allocations.

**But NOTE**: For initial implementation, **this phase has poor complexity vs. gain ratio**.  
Focus on **Moka cache first** (Phase 1 gives 15-20x vs Zero-Copy gives ~2x).

---

## Approach#

- Use `tokio-postgres` directly for hot paths (listing reads, search)
- **Not recommended for initial implementation** — complexity vs. gain is poor
- Focus on Moka cache first (Phase 1 gives 15-20x vs Zero-Copy gives ~2x)

---

## Step 2.1: Use tokio-postgres Directly#

**Only for hot paths** (listing reads, search):

```rust
// Instead of sqlx::query! macro, use tokio_postgres::Row
// and borrow &str from row buffers where possible

use tokio_postgres::{Client, NoTls};

pub struct ZeroCopyListing {
    listing_id: String,  // owned (primary key, small)
    product_name: String,  // owned (small field)
    // For large fields like description, borrow:
    // description: &'a str,  // borrowed from row buffer
}
```

---

## Step 2.2: Custom FromSql Implementation#

```rust
impl<'a> FromSql<'a> for ZeroCopyListing {
    fn from_sql(row: &'a Row) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(ZeroCopyListing {
            listing_id: row.try_get::<_, String>(0)?,
            product_name: row.try_get::<_, String>(1)?,
            // Avoid cloning large JSON fields if not needed
        })
    }
}
```

**Trade-off**: Complex to maintain. Only worthwhile if profiling shows `String` allocation is > 30% of CPU.

---

## Step 2.3: Configure Deadpool#

**File**: `backend/server/Cargo.toml`

```toml
[dependencies]
deadpool = "0.10"
deadpool-postgres = { version = "0.10", features = ["tokio"] }
```

**Configuration**:

```rust
use deadpool_postgres::{Config, Runtime};

let pool = Config {
    url: database_url,
    max_size: 20,  // Adjust based on server capacity
    timeout: Some(Duration::from_secs(30)),
    ..Default::default()
}
.create_pool(Some(Runtime::Tokio1))?;
```

**Expected Impact**: Better tail latency (p95/p99), minimal improvement on average ops/s.

---

## When to Implement#

| Condition | Action |
|-----------|--------|
| Phase 1 gives < 5,000 ops/s | **Skip Zero-Copy**, go straight to Deadpool |
| Allocation hotspots > 30% CPU (flamegraph) | Implement Zero-Copy for those paths |
| Tail latency (p99) > 10ms | Tune Deadpool settings |

---

## Files to Modify#

| File | Action | Purpose |
|------|--------|---------|
| `backend/server/Cargo.toml` | **Modify** | Add deadpool dependencies |
| `backend/server/src/app.rs` | **Modify** | update `build_postgres_app()` |
| `backend/server/src/repositories/listings.rs` | **Modify** | Optional: Zero-Copy rows |

---

## Expected Impact#

- **Deadpool alone**: Better tail latency (p95/p99), minimal improvement on average ops/s
- **Zero-Copy + Deadpool**: ~2x improvement (~5,000 → ~10,000 ops/s)
- **Trade-off**: High complexity, debuggability issues with borrowed references

---

## Risks & Mitigations#

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-------------|
| Zero-Copy borrows too complex | High | Medium | Fall back to owned strings if needed |
| Deadpool config too aggressive | Low | Low | Start with conservative `max_size: 10` |
| tokio-postgres API changes | Medium | High | Pin to exact version in Cargo.toml |

---

## Next Steps#

1. **[ ] Complete Phase 1 FIRST** — Actix + Moka (highest ROI)
2. **[ ] Profile with flamegraph** — identify if Zero-Copy is needed
3. **[ ] Implement Deadpool** — only after Phase 1 is stable
4. **[ ] Benchmark after each change** — document % improvement

---

**Document Status**: Phase 2 Details (Optional)    
**Last Updated**: 2026-05-07    
**Author**: pi (based on Phase 5 benchmark results)    
**Dependencies**: Phase 1 MUST be complete first  
