use crate::models::db::OutboxEventRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};
use async_trait::async_trait;
use sqlx::{postgres::PgPool, types::Json};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
};

#[async_trait]
pub trait OutboxEventRepository: Send + Sync {
    async fn append_event(&self, event: OutboxEventRow) -> Result<(), RepositoryError>;
}

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}

pub struct InMemoryOutboxEventRepository {
    events: RwLock<Vec<OutboxEventRow>>,
    next_event_id: AtomicU64,
}

impl InMemoryOutboxEventRepository {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
            next_event_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> i64 {
        self.next_event_id.fetch_add(1, Ordering::SeqCst) as i64
    }

    pub fn events(&self) -> Vec<OutboxEventRow> {
        self.events.read().expect("outbox events read lock").clone()
    }
}

impl Default for InMemoryOutboxEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutboxEventRepository for InMemoryOutboxEventRepository {
    async fn append_event(&self, mut event: OutboxEventRow) -> Result<(), RepositoryError> {
        if event.event_id <= 0 {
            event.event_id = self.next_id();
        }
        let mut events = self.events.write().expect("outbox events write lock");
        events.push(event);
        Ok(())
    }
}

pub struct PostgresOutboxEventRepository {
    pool: Arc<PgPool>,
}

impl PostgresOutboxEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub async fn append_event_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        event: OutboxEventRow,
    ) -> Result<(), RepositoryError> {
        let payload = Json(event.payload);
        sqlx::query(
            "INSERT INTO outbox_events (topic, aggregate_type, aggregate_id, payload, available_at, published_at, attempt_count, created_at) VALUES ($1,$2,$3,$4,$5::timestamptz,$6::timestamptz,$7,$8::timestamptz)",
        )
        .bind(event.topic)
        .bind(event.aggregate_type)
        .bind(event.aggregate_id)
        .bind(payload)
        .bind(event.available_at)
        .bind(event.published_at)
        .bind(event.attempt_count)
        .bind(event.created_at)
        .execute(&mut **tx)
        .await
        .map_err(|error| storage(error.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl OutboxEventRepository for PostgresOutboxEventRepository {
    async fn append_event(&self, event: OutboxEventRow) -> Result<(), RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|error| storage(error.to_string()))?;
        let payload = Json(event.payload);
        sqlx::query(
            "INSERT INTO outbox_events (topic, aggregate_type, aggregate_id, payload, available_at, published_at, attempt_count, created_at) VALUES ($1,$2,$3,$4,$5::timestamptz,$6::timestamptz,$7,$8::timestamptz)",
        )
        .bind(event.topic)
        .bind(event.aggregate_type)
        .bind(event.aggregate_id)
        .bind(payload)
        .bind(event.available_at)
        .bind(event.published_at)
        .bind(event.attempt_count)
        .bind(event.created_at)
        .execute(&mut *conn)
        .await
        .map_err(|error| storage(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn append_outbox_event_assigns_id_and_keeps_order() {
        let repo = InMemoryOutboxEventRepository::new();
        repo.append_event(OutboxEventRow {
            event_id: 0,
            topic: "listing.created".to_string(),
            aggregate_type: "listing".to_string(),
            aggregate_id: "lst_1".to_string(),
            payload: json!({"ok": true}),
            available_at: "2026-05-04T00:00:00Z".to_string(),
            published_at: None,
            attempt_count: 0,
            created_at: "2026-05-04T00:00:00Z".to_string(),
        })
        .await
        .unwrap();
        let events = repo.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, 1);
        assert_eq!(events[0].topic, "listing.created");
    }
}
