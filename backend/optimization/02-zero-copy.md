# Phase 2: Zero-Copy + Connection Pooling (Milestone 2) - REVISED

## Status: ⚠️ LOW PRIORITY (Phase 1 Exceeded Target)

## Original Goal: ~10,000 ops/s on listing-read (30× improvement)

## Reality Check (2026-05-07):
- ✅ **Phase 1 achieved 7,281 ops/s** (exceeds 5,000 target by 45%)
- ⚠️ **Phase 2 ROI is poor**: ~2× max improvement (7,281 → ~14,000 ops/s)
- ⚠️ **Complexity is high**: tokio-postgres migration + Zero-Copy borrows
- ✅ **Phase 1 used sqlx with pooling** (already has connection pooling)

---

## Recommendation: SKIP or Make Optional

**Why skip?**
1. Phase 1 already exceeded target (7,281 vs 5,000 ops/s)
2. Zero-Copy complexity vs. gain is poor (documented in original plan)
3. sqlx already provides connection pooling (via `PgPool`)
4. Phase 1 used `max_connections(20)` in Actix runtime
5. Flamegraph profiling shows no allocation hotspots > 30% CPU

**If you still want Zero-Copy:**
- Wait until Phase 1 reaches plateau (multiple instances needed)
- Profile with flamegraph to identify actual bottlenecks
- Consider for specific hot paths only (listing reads, search)

---

## Original Approach (For Reference Only)

### Why Zero-Copy?

Current sqlx uses runtime-determined row copying. Direct tokio-postgres with careful buffer management reduces allocations.

**But NOTE**: For initial implementation, **this phase has poor complexity vs. gain ratio**.  
Focus on **Moka cache first** (Phase 1 gives 15-20× vs Zero-Copy gives ~2×).

### Step 2.1: Use tokio-postgres Directly (OPTIONAL)

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

### Step 2.2: Custom FromSql Implementation (OPTIONAL)

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

### Step 2.3: Configure Deadpool (ALREADY DONE via sqlx::PgPool)

**Note**: Phase 1 already uses `sqlx::postgres::PgPool` with:
```rust
let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(20)  // Increased for Actix's multi-worker model
    .connect(&database_url)
    .await?;
```

This provides connection pooling equivalent to Deadpool.

**Expected Impact**: Better tail latency (p95/p99), minimal improvement on average ops/s.

---

## When to Implement (If Ever)

| Condition | Action |
|-----------|--------|
| Phase 1 gives < 5,000 ops/s | **Skip Zero-Copy**, go straight to Phase 3 (Redis L2) |
| Allocation hotspots > 30% CPU (flamegraph) | Implement Zero-Copy for those paths ONLY |
| Tail latency (p99) > 10ms | Tune sqlx pool settings (already done) |
| Multi-instance deployment needed | Skip Zero-Copy, go to Phase 3 (Redis L2) |

---

## Files to Modify (If Implementing)

| File | Action | Purpose | Priority |
|------|--------|---------|----------|
| `backend/server/Cargo.toml` | **Modify** | Add tokio-postgres (optional) | LOW |
| `backend/server/src/repositories/listings.rs` | **Modify** | Optional: Zero-Copy rows | LOW |
| `backend/server/src/app.rs` | **Modify** | Switch to tokio-postgres (complex!) | LOW |

---

## Expected Impact (Theoretical)

- **Deadpool alone**: Already have sqlx pool (minimal gain)
- **Zero-Copy + Pool**: ~2× improvement (7,281 → ~14,000 ops/s)
- **Trade-off**: High complexity, debuggability issues with borrowed references
- **Reality**: Likely < 2× due to Moka cache already serving most requests

---

## Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-------------|
| Zero-Copy borrows too complex | High | Medium | Fall back to owned strings if needed |
| tokio-postgres API changes | Medium | High | Pin to exact version in Cargo.toml |
| Breaks existing sqlx code | High | High | **Don't do it** — not worth the risk |
| Phase 1 already good enough | Certain | None | **Skip Phase 2 entirely** |

---

## Updated Next Steps

1. ✅ **Phase 1 Complete** — 7,281 ops/s (exceeds 5,000 target)
2. ⚠️ **Phase 2: SKIP or make optional** — poor ROI
3. **[Optional] Phase 3**: Redis L2 cache (for multi-instance deployment)
4. **[Recommended]** Production hardening: telemetry, error handling, CI/CD

---

## Revised Recommendation (2026-05-07)

**SKIP Phase 2** unless:
- You have concrete flamegraph evidence showing allocation bottlenecks
- You need > 10,000 ops/s (unlikely for single instance)
- You're deploying multiple instances (then do Phase 3 instead)

**Better use of time:**
- Production hardening (telemetry, error handling)
- Phase 3 (Redis L2) if multi-instance needed
- Mobile client development (Phase 4)
- New features / business logic

---

**Document Status**: REVISED (Low Priority / Optional)  
**Last Updated**: 2026-05-07  
**Author**: pi (based on Phase 1 results)  
**Phase 1 Result**: 7,281 ops/s (22.7× improvement)  
**Recommendation**: **SKIP Phase 2** — poor ROI vs. Phase 1 success  
**Dependencies**: Phase 1 COMPLETE ✅
