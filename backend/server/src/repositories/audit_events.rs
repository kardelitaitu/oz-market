use crate::models::db::AuditEventRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};
use async_trait::async_trait;
use sqlx::{postgres::PgPool, types::Json};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
};

#[async_trait]
pub trait AuditEventRepository: Send + Sync {
    async fn append_event(&self, event: AuditEventRow) -> Result<(), RepositoryError>;
}

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}

pub struct InMemoryAuditEventRepository {
    events: RwLock<Vec<AuditEventRow>>,
    next_event_id: AtomicU64,
}

impl InMemoryAuditEventRepository {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
            next_event_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> i64 {
        self.next_event_id.fetch_add(1, Ordering::SeqCst) as i64
    }

    pub fn events(&self) -> Vec<AuditEventRow> {
        self.events.read().expect("audit events read lock").clone()
    }
}

impl Default for InMemoryAuditEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditEventRepository for InMemoryAuditEventRepository {
    async fn append_event(&self, mut event: AuditEventRow) -> Result<(), RepositoryError> {
        if event.event_id <= 0 {
            event.event_id = self.next_id();
        }
        let mut events = self.events.write().expect("audit events write lock");
        events.push(event);
        Ok(())
    }
}

pub struct PostgresAuditEventRepository {
    pool: Arc<PgPool>,
}

impl PostgresAuditEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub async fn append_event_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        event: AuditEventRow,
    ) -> Result<(), RepositoryError> {
        let scopes = Json(event.scopes);
        let payload = Json(event.payload);
        sqlx::query(
            "INSERT INTO audit_events (entity_type, entity_id, action, actor_subject, actor_role, scopes, request_id, idempotency_key, payload, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10::timestamptz)",
        )
        .bind(event.entity_type)
        .bind(event.entity_id)
        .bind(event.action)
        .bind(event.actor_subject)
        .bind(event.actor_role)
        .bind(scopes)
        .bind(event.request_id)
        .bind(event.idempotency_key)
        .bind(payload)
        .bind(event.created_at)
        .execute(&mut **tx)
        .await
        .map_err(|error| storage(error.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl AuditEventRepository for PostgresAuditEventRepository {
    async fn append_event(&self, event: AuditEventRow) -> Result<(), RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|error| storage(error.to_string()))?;
        let scopes = Json(event.scopes);
        let payload = Json(event.payload);
        sqlx::query(
            "INSERT INTO audit_events (entity_type, entity_id, action, actor_subject, actor_role, scopes, request_id, idempotency_key, payload, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10::timestamptz)",
        )
        .bind(event.entity_type)
        .bind(event.entity_id)
        .bind(event.action)
        .bind(event.actor_subject)
        .bind(event.actor_role)
        .bind(scopes)
        .bind(event.request_id)
        .bind(event.idempotency_key)
        .bind(payload)
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
    async fn append_audit_event_assigns_id_and_keeps_order() {
        let repo = InMemoryAuditEventRepository::new();
        repo.append_event(AuditEventRow {
            event_id: 0,
            entity_type: "listing".to_string(),
            entity_id: "lst_1".to_string(),
            action: "create".to_string(),
            actor_subject: "sub-1".to_string(),
            actor_role: "SellerListingWriter".to_string(),
            scopes: vec!["listing:create".to_string()],
            request_id: None,
            idempotency_key: Some("idem-1".to_string()),
            payload: json!({"ok": true}),
            created_at: "2026-05-04T00:00:00Z".to_string(),
        })
        .await
        .unwrap();
        let events = repo.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, 1);
        assert_eq!(events[0].entity_id, "lst_1");
    }
}
