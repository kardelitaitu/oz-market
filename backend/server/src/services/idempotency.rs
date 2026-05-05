use crate::models::db::{IdempotencyKeyRow, IdempotencyKeyStatus};
use crate::repositories::{IdempotencyKeyRepository, RepositoryError};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdempotencyOperation {
    CreateListing,
    OpenNegotiation,
    SubmitOffer,
    RequestContactReveal,
    ApproveContactReveal,
}

impl IdempotencyOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CreateListing => "create_listing",
            Self::OpenNegotiation => "open_negotiation",
            Self::SubmitOffer => "submit_offer",
            Self::RequestContactReveal => "request_contact_reveal",
            Self::ApproveContactReveal => "approve_contact_reveal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyAttempt<'a> {
    pub actor_subject: &'a str,
    pub operation: IdempotencyOperation,
    pub idempotency_key: &'a str,
    pub request_fingerprint: &'a str,
    pub ttl_seconds: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IdempotencyDecision {
    FirstUse,
    ReplayAccepted { response_payload: Option<Value> },
    InFlight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyErrorKind {
    InvalidKey,
    Conflict,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyError {
    pub kind: IdempotencyErrorKind,
    pub message: String,
}

impl IdempotencyError {
    pub fn new(kind: IdempotencyErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl Display for IdempotencyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for IdempotencyError {}

pub struct IdempotencyGuard<R> {
    repository: Arc<R>,
}

impl<R> IdempotencyGuard<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> IdempotencyGuard<R>
where
    R: IdempotencyKeyRepository + Send + Sync,
{
    pub async fn begin(
        &self,
        attempt: &IdempotencyAttempt<'_>,
        now_rfc3339: &str,
    ) -> Result<IdempotencyDecision, IdempotencyError> {
        validate_attempt(attempt)?;

        match self
            .repository
            .as_ref()
            .get(
                attempt.actor_subject,
                attempt.operation.as_str(),
                attempt.idempotency_key,
            )
            .await
            .map_err(map_repo_error)?
        {
            Some(existing) if existing.request_fingerprint == attempt.request_fingerprint => {
                match existing.status {
                    IdempotencyKeyStatus::Succeeded => Ok(IdempotencyDecision::ReplayAccepted {
                        response_payload: existing.response_payload,
                    }),
                    IdempotencyKeyStatus::Pending => Ok(IdempotencyDecision::InFlight),
                    IdempotencyKeyStatus::Failed => Ok(IdempotencyDecision::ReplayAccepted {
                        response_payload: existing.response_payload,
                    }),
                }
            }
            Some(_) => Err(IdempotencyError::new(
                IdempotencyErrorKind::Conflict,
                "idempotency key reused with a different request fingerprint",
            )),
            None => {
                self.repository
                    .as_ref()
                    .reserve(IdempotencyKeyRow {
                        idempotency_key: attempt.idempotency_key.to_string(),
                        actor_subject: attempt.actor_subject.to_string(),
                        operation: attempt.operation.as_str().to_string(),
                        request_fingerprint: attempt.request_fingerprint.to_string(),
                        status: IdempotencyKeyStatus::Pending,
                        response_payload: None,
                        expires_at: compute_expiry(now_rfc3339, attempt.ttl_seconds),
                        created_at: now_rfc3339.to_string(),
                        updated_at: now_rfc3339.to_string(),
                    })
                    .await
                    .map_err(map_repo_error)?;

                Ok(IdempotencyDecision::FirstUse)
            }
        }
    }

    pub async fn commit_success(
        &self,
        attempt: &IdempotencyAttempt<'_>,
        response_payload: Value,
    ) -> Result<(), IdempotencyError> {
        self.repository
            .as_ref()
            .mark_succeeded(
                attempt.actor_subject,
                attempt.operation.as_str(),
                attempt.idempotency_key,
                response_payload,
            )
            .await
            .map_err(map_repo_error)
    }

    pub async fn commit_failure(
        &self,
        attempt: &IdempotencyAttempt<'_>,
        response_payload: Option<Value>,
    ) -> Result<(), IdempotencyError> {
        self.repository
            .as_ref()
            .mark_failed(
                attempt.actor_subject,
                attempt.operation.as_str(),
                attempt.idempotency_key,
                response_payload,
            )
            .await
            .map_err(map_repo_error)
    }
}

fn validate_attempt(attempt: &IdempotencyAttempt<'_>) -> Result<(), IdempotencyError> {
    if attempt.idempotency_key.trim().is_empty() {
        return Err(IdempotencyError::new(
            IdempotencyErrorKind::InvalidKey,
            "idempotency_key must not be empty",
        ));
    }
    if attempt.actor_subject.trim().is_empty() {
        return Err(IdempotencyError::new(
            IdempotencyErrorKind::InvalidKey,
            "actor_subject must not be empty",
        ));
    }
    if attempt.request_fingerprint.trim().is_empty() {
        return Err(IdempotencyError::new(
            IdempotencyErrorKind::InvalidKey,
            "request_fingerprint must not be empty",
        ));
    }
    if attempt.ttl_seconds <= 0 {
        return Err(IdempotencyError::new(
            IdempotencyErrorKind::InvalidKey,
            "ttl_seconds must be positive",
        ));
    }
    Ok(())
}

fn compute_expiry(now_rfc3339: &str, ttl_seconds: i64) -> String {
    format!("{now_rfc3339}+{ttl_seconds}s")
}

fn map_repo_error(error: RepositoryError) -> IdempotencyError {
    IdempotencyError::new(IdempotencyErrorKind::Storage, error.to_string())
}

pub struct InMemoryIdempotencyRepository {
    records: RwLock<HashMap<(String, String, String), IdempotencyKeyRow>>,
}

impl InMemoryIdempotencyRepository {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryIdempotencyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IdempotencyKeyRepository for InMemoryIdempotencyRepository {
    async fn get(
        &self,
        actor_subject: &str,
        operation: &str,
        idempotency_key: &str,
    ) -> Result<Option<IdempotencyKeyRow>, RepositoryError> {
        let guard = self.records.read().expect("idempotency read lock");
        Ok(guard
            .get(&(
                actor_subject.to_string(),
                operation.to_string(),
                idempotency_key.to_string(),
            ))
            .cloned())
    }

    async fn reserve(&self, record: IdempotencyKeyRow) -> Result<(), RepositoryError> {
        let mut guard = self.records.write().expect("idempotency write lock");
        let key = (
            record.actor_subject.clone(),
            record.operation.clone(),
            record.idempotency_key.clone(),
        );
        if guard.contains_key(&key) {
            return Err(RepositoryError::new(
                crate::repositories::RepositoryErrorKind::Conflict,
                "idempotency record already exists",
            ));
        }
        guard.insert(key, record);
        Ok(())
    }

    async fn mark_succeeded(
        &self,
        actor_subject: &str,
        operation: &str,
        idempotency_key: &str,
        response_payload: Value,
    ) -> Result<(), RepositoryError> {
        let mut guard = self.records.write().expect("idempotency write lock");
        let key = (
            actor_subject.to_string(),
            operation.to_string(),
            idempotency_key.to_string(),
        );
        let record = guard.get_mut(&key).ok_or_else(|| {
            RepositoryError::new(
                crate::repositories::RepositoryErrorKind::NotFound,
                "idempotency record not found",
            )
        })?;
        record.status = IdempotencyKeyStatus::Succeeded;
        record.response_payload = Some(response_payload);
        Ok(())
    }

    async fn mark_failed(
        &self,
        actor_subject: &str,
        operation: &str,
        idempotency_key: &str,
        response_payload: Option<Value>,
    ) -> Result<(), RepositoryError> {
        let mut guard = self.records.write().expect("idempotency write lock");
        let key = (
            actor_subject.to_string(),
            operation.to_string(),
            idempotency_key.to_string(),
        );
        let record = guard.get_mut(&key).ok_or_else(|| {
            RepositoryError::new(
                crate::repositories::RepositoryErrorKind::NotFound,
                "idempotency record not found",
            )
        })?;
        record.status = IdempotencyKeyStatus::Failed;
        record.response_payload = response_payload;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt() -> IdempotencyAttempt<'static> {
        IdempotencyAttempt {
            actor_subject: "sub-1",
            operation: IdempotencyOperation::CreateListing,
            idempotency_key: "key-1",
            request_fingerprint: "fp-1",
            ttl_seconds: 3600,
        }
    }

    #[tokio::test]
    async fn first_use_reserves_record() {
        let repo = InMemoryIdempotencyRepository::new();
        let guard = IdempotencyGuard::new(Arc::new(repo));
        let decision = guard
            .begin(&attempt(), "2026-05-04T00:00:00Z")
            .await
            .unwrap();
        assert!(matches!(decision, IdempotencyDecision::FirstUse));
    }

    #[tokio::test]
    async fn replay_same_fingerprint_returns_accepted() {
        let repo = InMemoryIdempotencyRepository::new();
        let guard = IdempotencyGuard::new(Arc::new(repo));
        let attempt = attempt();
        let _ = guard.begin(&attempt, "2026-05-04T00:00:00Z").await.unwrap();
        guard
            .commit_success(&attempt, serde_json::json!({"listing_id": "lst_1"}))
            .await
            .unwrap();
        let decision = guard.begin(&attempt, "2026-05-04T00:00:01Z").await.unwrap();
        match decision {
            IdempotencyDecision::ReplayAccepted { response_payload } => {
                assert!(response_payload.is_some());
            }
            _ => panic!("expected replay accepted"),
        }
    }

    #[tokio::test]
    async fn reused_key_with_different_fingerprint_conflicts() {
        let repo = InMemoryIdempotencyRepository::new();
        let guard = IdempotencyGuard::new(Arc::new(repo));
        let attempt = attempt();
        let _ = guard.begin(&attempt, "2026-05-04T00:00:00Z").await.unwrap();
        let conflicting = IdempotencyAttempt {
            request_fingerprint: "fp-2",
            ..attempt
        };
        let decision = guard.begin(&conflicting, "2026-05-04T00:00:01Z").await;
        assert!(matches!(
            decision,
            Err(IdempotencyError {
                kind: IdempotencyErrorKind::Conflict,
                ..
            })
        ));
    }
}
