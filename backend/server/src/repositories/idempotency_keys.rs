use crate::models::db::{IdempotencyKeyRow, IdempotencyKeyStatus};
use crate::repositories::{RepositoryError, RepositoryErrorKind};
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait IdempotencyKeyRepository: Send + Sync {
    async fn get(
        &self,
        actor_subject: &str,
        operation: &str,
        idempotency_key: &str,
    ) -> Result<Option<IdempotencyKeyRow>, RepositoryError>;

    async fn reserve(
        &self,
        record: IdempotencyKeyRow,
    ) -> Result<(), RepositoryError>;

    async fn mark_succeeded(
        &self,
        actor_subject: &str,
        operation: &str,
        idempotency_key: &str,
        response_payload: Value,
    ) -> Result<(), RepositoryError>;

    async fn mark_failed(
        &self,
        actor_subject: &str,
        operation: &str,
        idempotency_key: &str,
        response_payload: Option<Value>,
    ) -> Result<(), RepositoryError>;
}

pub fn status_is_terminal(status: IdempotencyKeyStatus) -> bool {
    matches!(status, IdempotencyKeyStatus::Succeeded | IdempotencyKeyStatus::Failed)
}

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}
