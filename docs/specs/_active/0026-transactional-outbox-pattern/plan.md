# Plan - Transactional Outbox Pattern

## Implementation Steps

1. **Migration Schema**:
   - Create migration adding `status` (default: `'pending'`) and `published_at` columns to `outbox_events` table.

2. **Transactional Repository Integration**:
   - Update `OutboxEventRepository` to support inserting event records inside an active `sqlx::Transaction` block.

3. **Background Publisher Worker**:
   - Create `backend/server/src/services/outbox_publisher.rs`.
   - Setup a `tokio` background loop polling the outbox table using `FOR UPDATE SKIP LOCKED`.
   - Call external dispatchers or broadcast channels. On success, update the outbox status to `published`.

4. **Sweep Cron Task**:
   - Implement clean up query deleting events that have been `published` for more than 24 hours.
