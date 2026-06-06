# Decisions - Delete Legacy Benchmarks

## Architecture Decisions

### 1. Complete Removal of Fragmented Scripts
- **Decision**: All legacy benchmark targets and scripts will be permanently deleted from the source tree rather than commented out.
- **Rationale**: Removes technical debt and prevents confusion for subsequent developer runs, ensuring the unified benchmark suite is the sole testing protocol.

### 2. Keep `populate_db.rs`
- **Decision**: Keep the `populate_db.rs` binary configuration.
- **Rationale**: Although populate_db is used to seed databases for benchmarks, it is also required to bootstrap local developer database setups.
