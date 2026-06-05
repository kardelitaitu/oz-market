# Implementation Notes - Transactional Outbox Pattern

## Database Outbox Polling Query

```sql
-- Fetch pending events skipping locked rows
BEGIN;
SELECT id, event_type, payload 
FROM outbox_events 
WHERE status = 'pending' 
ORDER BY created_at ASC 
LIMIT 100 
FOR UPDATE SKIP LOCKED;

-- Update published status after successfully sending
UPDATE outbox_events 
SET status = 'published', published_at = NOW() 
WHERE id = ANY($1);
COMMIT;
```

## Background Task Scheduler

```rust
use std::sync::Arc;
use tokio::time::{interval, Duration};

pub async fn start_outbox_publisher(
    pool: sqlx::PgPool,
    notifier: Arc<dyn EventNotifier>,
) {
    let mut interval_timer = interval(Duration::from_millis(500));
    loop {
        interval_timer.tick().await;
        let batch = match fetch_pending_batch(&pool).await {
            Ok(events) => events,
            Err(_) => continue,
        };

        if batch.is_empty() {
            continue;
        }

        let mut published_ids = Vec::new();
        for event in batch {
            // Dispatch domain event to broadcasting channel or SSE service
            if notifier.publish(&event.event_type, &event.payload).await.is_ok() {
                published_ids.push(event.id);
            }
        }

        if !published_ids.is_empty() {
            let _ = mark_as_published(&pool, &published_ids).await;
        }
    }
}
```
