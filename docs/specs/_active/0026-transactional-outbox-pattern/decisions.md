# Decisions - Transactional Outbox Pattern

## Architecture Decisions

### 1. `SKIP LOCKED` for Concurrent Publishing
- **Decision**: The polling query must use `FOR UPDATE SKIP LOCKED`.
- **Rationale**: Prevents lock contention and double-publishing if multiple server instances run the polling worker concurrently.

### 2. Auto-sweeping Stale Logs
- **Decision**: Keep published records in the database for 24 hours before deleting them via background sweep cron tasks.
- **Rationale**: Retains immediate auditing context for local troubleshooting while preventing the database index from bloating.
