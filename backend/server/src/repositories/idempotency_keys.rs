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

    async fn reserve(&self, record: IdempotencyKeyRow) -> Result<(), RepositoryError>;

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
    matches!(
        status,
        IdempotencyKeyStatus::Succeeded | IdempotencyKeyStatus::Failed
    )
}

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}

// --- Postgres implementation ---

pub struct PostgresIdempotencyKeyRepository {
    pool: sqlx::postgres::PgPool,
}

impl PostgresIdempotencyKeyRepository {
    pub fn new(pool: sqlx::postgres::PgPool) -> Self {
        Self { pool }
    }
}

fn idempotency_status_as_str(s: IdempotencyKeyStatus) -> &'static str {
    match s {
        IdempotencyKeyStatus::Pending => "pending",
        IdempotencyKeyStatus::Succeeded => "succeeded",
        IdempotencyKeyStatus::Failed => "failed",
    }
}

fn idempotency_status_from_str(s: &str) -> Result<IdempotencyKeyStatus, RepositoryError> {
    match s {
        "pending" => Ok(IdempotencyKeyStatus::Pending),
        "succeeded" => Ok(IdempotencyKeyStatus::Succeeded),
        "failed" => Ok(IdempotencyKeyStatus::Failed),
        _ => Err(RepositoryError::new(
            RepositoryErrorKind::Storage,
            format!("invalid idempotency status: {s}"),
        )),
    }
}

fn row_from_pg_row(row: sqlx::postgres::PgRow) -> Result<IdempotencyKeyRow, RepositoryError> {
    use sqlx::Row;
    Ok(IdempotencyKeyRow {
        idempotency_key: row.get("idempotency_key"),
        actor_subject: row.get("actor_subject"),
        operation: row.get("operation"),
        request_fingerprint: row.get("request_fingerprint"),
        status: idempotency_status_from_str(row.get::<String, _>("status").as_str())?,
        response_payload: row.get("response_payload"),
        expires_at: row.get("expires_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[async_trait]
impl IdempotencyKeyRepository for PostgresIdempotencyKeyRepository {
    async fn get(
        &self,
        actor_subject: &str,
        operation: &str,
        idempotency_key: &str,
    ) -> Result<Option<IdempotencyKeyRow>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let row = sqlx::query(
            "SELECT idempotency_key, actor_subject, operation, request_fingerprint, status, \
             response_payload, expires_at::TEXT AS expires_at, created_at::TEXT AS created_at, updated_at::TEXT AS updated_at \
             FROM idempotency_keys \
             WHERE idempotency_key = $1 AND actor_subject = $2 AND operation = $3",
        )
        .bind(idempotency_key)
        .bind(actor_subject)
        .bind(operation)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        match row {
            Some(r) => Ok(Some(row_from_pg_row(r)?)),
            None => Ok(None),
        }
    }

    async fn reserve(&self, record: IdempotencyKeyRow) -> Result<(), RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let status = idempotency_status_as_str(record.status);

        sqlx::query(
            "INSERT INTO idempotency_keys (idempotency_key, actor_subject, operation, request_fingerprint, status, response_payload, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, now() + interval '1 day') \
             ON CONFLICT (idempotency_key, actor_subject, operation) DO NOTHING",
        )
        .bind(&record.idempotency_key)
        .bind(&record.actor_subject)
        .bind(&record.operation)
        .bind(&record.request_fingerprint)
        .bind(status)
        .bind(&record.response_payload)
        .execute(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        Ok(())
    }

    async fn mark_succeeded(
        &self,
        actor_subject: &str,
        operation: &str,
        idempotency_key: &str,
        response_payload: Value,
    ) -> Result<(), RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let result = sqlx::query(
            "UPDATE idempotency_keys SET status = 'succeeded', response_payload = $1, updated_at = now() \
             WHERE idempotency_key = $2 AND actor_subject = $3 AND operation = $4",
        )
        .bind(&response_payload)
        .bind(idempotency_key)
        .bind(actor_subject)
        .bind(operation)
        .execute(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::new(
                RepositoryErrorKind::NotFound,
                "idempotency record not found",
            ));
        }
        Ok(())
    }

    async fn mark_failed(
        &self,
        actor_subject: &str,
        operation: &str,
        idempotency_key: &str,
        response_payload: Option<Value>,
    ) -> Result<(), RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let result = sqlx::query(
            "UPDATE idempotency_keys SET status = 'failed', response_payload = $1, updated_at = now() \
             WHERE idempotency_key = $2 AND actor_subject = $3 AND operation = $4",
        )
        .bind(&response_payload)
        .bind(idempotency_key)
        .bind(actor_subject)
        .bind(operation)
        .execute(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::new(
                RepositoryErrorKind::NotFound,
                "idempotency record not found",
            ));
        }
        Ok(())
    }
}
