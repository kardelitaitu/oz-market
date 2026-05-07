use crate::models::db::ContactRevealRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};
use async_trait::async_trait;
use marketplace_api_contract::{
    ContactRevealResponse, ContactRevealStatus, RequestContactRevealRequest,
};
use sqlx::{
    postgres::{PgPool, PgRow},
    Row,
};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
};

#[async_trait]
pub trait ContactRevealRepository: Send + Sync {
    async fn create_request(
        &self,
        negotiation_id: &str,
        request: &RequestContactRevealRequest,
        buyer_agent_id: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, RepositoryError>;

    async fn approve_request(
        &self,
        reveal_id: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, RepositoryError>;

    async fn get_by_reveal_id(
        &self,
        reveal_id: &str,
    ) -> Result<Option<ContactRevealResponse>, RepositoryError>;
}

pub fn conflict(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Conflict, message)
}

pub fn not_found(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::NotFound, message)
}

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}

fn parse_reveal_status(status: &str) -> Result<ContactRevealStatus, RepositoryError> {
    match status {
        "pending" => Ok(ContactRevealStatus::Pending),
        "approved" => Ok(ContactRevealStatus::Approved),
        "rejected" => Ok(ContactRevealStatus::Rejected),
        "expired" => Ok(ContactRevealStatus::Expired),
        _ => Err(RepositoryError::new(
            RepositoryErrorKind::Storage,
            "invalid reveal status",
        )),
    }
}

pub struct InMemoryContactRevealRepository {
    by_reveal_id: RwLock<HashMap<String, ContactRevealRow>>,
    by_negotiation_id: RwLock<HashMap<String, String>>,
    next_reveal_id: AtomicU64,
}

impl InMemoryContactRevealRepository {
    pub fn new() -> Self {
        Self {
            by_reveal_id: RwLock::new(HashMap::new()),
            by_negotiation_id: RwLock::new(HashMap::new()),
            next_reveal_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> String {
        let next = self.next_reveal_id.fetch_add(1, Ordering::SeqCst);
        format!("rev_{next:06}")
    }

    fn row_to_response(row: &ContactRevealRow) -> ContactRevealResponse {
        ContactRevealResponse {
            reveal_id: row.reveal_id.clone(),
            negotiation_id: row.negotiation_id.clone(),
            reveal_status: row.reveal_status,
            revealed_phone_reference: row.revealed_phone_reference.clone(),
            expires_at: row.expires_at.clone(),
            approved_at: row.approved_at.clone(),
            updated_at: row.updated_at.clone(),
        }
    }
}

impl Default for InMemoryContactRevealRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContactRevealRepository for InMemoryContactRevealRepository {
    async fn create_request(
        &self,
        negotiation_id: &str,
        request: &RequestContactRevealRequest,
        buyer_agent_id: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, RepositoryError> {
        let mut by_negotiation = self
            .by_negotiation_id
            .write()
            .expect("contact reveal negotiation write lock");
        let mut by_reveal = self
            .by_reveal_id
            .write()
            .expect("contact reveal write lock");

        if let Some(existing_reveal_id) = by_negotiation.get(negotiation_id) {
            if let Some(existing) = by_reveal.get(existing_reveal_id) {
                return Ok(Self::row_to_response(existing));
            }
        }

        let listing_id = negotiation_id
            .strip_prefix("neg_")
            .unwrap_or(negotiation_id);

        let row = ContactRevealRow {
            reveal_id: self.next_id(),
            negotiation_id: negotiation_id.to_string(),
            listing_id: listing_id.to_string(),
            buyer_agent_id: buyer_agent_id.to_string(),
            request_idempotency_key: request.idempotency_key.clone(),
            reveal_status: ContactRevealStatus::Pending,
            revealed_phone_reference: None,
            expires_at: Some(format!("{now_rfc3339}+900s")),
            approved_at: None,
            created_at: now_rfc3339.to_string(),
            updated_at: now_rfc3339.to_string(),
        };

        by_negotiation.insert(negotiation_id.to_string(), row.reveal_id.clone());
        by_reveal.insert(row.reveal_id.clone(), row.clone());
        Ok(Self::row_to_response(&row))
    }

    async fn approve_request(
        &self,
        reveal_id: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, RepositoryError> {
        let mut by_reveal = self
            .by_reveal_id
            .write()
            .expect("contact reveal write lock");
        let row = by_reveal
            .get_mut(reveal_id)
            .ok_or_else(|| not_found("contact reveal not found"))?;
        if row.reveal_status != ContactRevealStatus::Pending {
            return Err(conflict("contact reveal is not pending"));
        }
        row.reveal_status = ContactRevealStatus::Approved;
        row.revealed_phone_reference = Some("phone_ref_stub".to_string());
        row.approved_at = Some(now_rfc3339.to_string());
        row.updated_at = now_rfc3339.to_string();
        Ok(Self::row_to_response(row))
    }

    async fn get_by_reveal_id(
        &self,
        reveal_id: &str,
    ) -> Result<Option<ContactRevealResponse>, RepositoryError> {
        let by_reveal = self.by_reveal_id.read().expect("contact reveal read lock");
        Ok(by_reveal.get(reveal_id).map(Self::row_to_response))
    }
}

pub struct PostgresContactRevealRepository {
    pool: Arc<PgPool>,
}

impl PostgresContactRevealRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    fn row_to_response(row: &PgRow) -> Result<ContactRevealResponse, RepositoryError> {
        let reveal_status: String = row
            .try_get("reveal_status")
            .map_err(|error| storage(error.to_string()))?;
        let reveal_status = parse_reveal_status(reveal_status.as_str())?;

        Ok(ContactRevealResponse {
            reveal_id: row
                .try_get("reveal_id")
                .map_err(|error| storage(error.to_string()))?,
            negotiation_id: row
                .try_get("negotiation_id")
                .map_err(|error| storage(error.to_string()))?,
            reveal_status,
            revealed_phone_reference: row
                .try_get("revealed_phone_reference")
                .map_err(|error| storage(error.to_string()))?,
            expires_at: row
                .try_get("expires_at")
                .map_err(|error| storage(error.to_string()))?,
            approved_at: row
                .try_get("approved_at")
                .map_err(|error| storage(error.to_string()))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|error| storage(error.to_string()))?,
        })
    }

    async fn next_reveal_id(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<String, RepositoryError> {
        let next = sqlx::query_scalar::<_, i64>("SELECT nextval('contact_reveal_id_seq')")
            .fetch_one(&mut **tx)
            .await
            .map_err(|error| storage(error.to_string()))?;
        Ok(format!("rev_{next:06}"))
    }
}

#[async_trait]
impl ContactRevealRepository for PostgresContactRevealRepository {
    async fn create_request(
        &self,
        negotiation_id: &str,
        request: &RequestContactRevealRequest,
        buyer_agent_id: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| storage(error.to_string()))?;

        let negotiation_row =
            sqlx::query("SELECT listing_id FROM reservation_leases WHERE negotiation_id = $1 FOR UPDATE")
                .bind(negotiation_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|error| storage(error.to_string()))?;
        let Some(negotiation_row) = negotiation_row else {
            return Err(not_found("negotiation not found"));
        };
        let listing_id: String = negotiation_row
            .try_get("listing_id")
            .map_err(|error| storage(error.to_string()))?;

        let existing_row = sqlx::query(
            "SELECT reveal_id, negotiation_id, listing_id, buyer_agent_id, request_idempotency_key, reveal_status, revealed_phone_reference, to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at, to_char(approved_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS approved_at, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at FROM contact_reveals WHERE negotiation_id = $1 LIMIT 1",
        )
        .bind(negotiation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| storage(error.to_string()))?;
        if let Some(existing_row) = existing_row {
            let row = Self::row_to_response(&existing_row)?;
            tx.commit()
                .await
                .map_err(|error| storage(error.to_string()))?;
            return Ok(row);
        }

        let reveal_id = Self::next_reveal_id(&mut tx).await?;
        let row = sqlx::query(
            "INSERT INTO contact_reveals (reveal_id, negotiation_id, listing_id, buyer_agent_id, request_idempotency_key, reveal_status, revealed_phone_reference, expires_at, approved_at, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, 'pending', NULL, ($6::timestamptz + interval '900 seconds'), NULL, $6::timestamptz, $6::timestamptz) RETURNING reveal_id, negotiation_id, listing_id, buyer_agent_id, request_idempotency_key, reveal_status, revealed_phone_reference, to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at, to_char(approved_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS approved_at, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at",
        )
        .bind(&reveal_id)
        .bind(negotiation_id)
        .bind(&listing_id)
        .bind(buyer_agent_id)
        .bind(&request.idempotency_key)
        .bind(now_rfc3339)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| storage(error.to_string()))?;

        tx.commit()
            .await
            .map_err(|error| storage(error.to_string()))?;
        Self::row_to_response(&row)
    }

    async fn approve_request(
        &self,
        reveal_id: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| storage(error.to_string()))?;

        let current = sqlx::query(
            "SELECT reveal_status FROM contact_reveals WHERE reveal_id = $1 FOR UPDATE",
        )
        .bind(reveal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| storage(error.to_string()))?;

        let Some(current) = current else {
            return Err(not_found("contact reveal not found"));
        };
        let status: String = current
            .try_get("reveal_status")
            .map_err(|error| storage(error.to_string()))?;
        if status != "pending" {
            return Err(conflict("contact reveal is not pending"));
        }

        let row = sqlx::query(
            "UPDATE contact_reveals SET reveal_status = 'approved', revealed_phone_reference = 'phone_ref_stub', approved_at = $2::timestamptz, updated_at = $2::timestamptz WHERE reveal_id = $1 RETURNING reveal_id, negotiation_id, listing_id, buyer_agent_id, request_idempotency_key, reveal_status, revealed_phone_reference, to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at, to_char(approved_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS approved_at, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at",
        )
        .bind(reveal_id)
        .bind(now_rfc3339)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| storage(error.to_string()))?;

        tx.commit()
            .await
            .map_err(|error| storage(error.to_string()))?;
        Self::row_to_response(&row)
    }

    async fn get_by_reveal_id(
        &self,
        reveal_id: &str,
    ) -> Result<Option<ContactRevealResponse>, RepositoryError> {
        let row = sqlx::query(
            "SELECT reveal_id, negotiation_id, listing_id, buyer_agent_id, request_idempotency_key, reveal_status, revealed_phone_reference, to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at, to_char(approved_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS approved_at, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at FROM contact_reveals WHERE reveal_id = $1",
        )
        .bind(reveal_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|error| storage(error.to_string()))?;

        row.as_ref().map(Self::row_to_response).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_and_approve_contact_reveal() {
        let repo = InMemoryContactRevealRepository::new();
        let create = repo
            .create_request(
                "neg_1",
                &RequestContactRevealRequest {
                    idempotency_key: "idem-1".to_string(),
                },
                "buyer-1",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();
        assert_eq!(create.reveal_status, ContactRevealStatus::Pending);

        let approved = repo
            .approve_request(&create.reveal_id, "2026-05-04T00:01:00Z")
            .await
            .unwrap();
        assert_eq!(approved.reveal_status, ContactRevealStatus::Approved);
        assert!(approved.revealed_phone_reference.is_some());
    }

    #[tokio::test]
    async fn approve_rejects_non_pending_reveal() {
        let repo = InMemoryContactRevealRepository::new();
        let create = repo
            .create_request(
                "neg_1",
                &RequestContactRevealRequest {
                    idempotency_key: "idem-1".to_string(),
                },
                "buyer-1",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();
        let first = repo
            .approve_request(&create.reveal_id, "2026-05-04T00:01:00Z")
            .await
            .unwrap();
        assert_eq!(first.reveal_status, ContactRevealStatus::Approved);

        let second = repo
            .approve_request(&create.reveal_id, "2026-05-04T00:02:00Z")
            .await;
        assert!(matches!(
            second,
            Err(RepositoryError {
                kind: RepositoryErrorKind::Conflict,
                ..
            })
        ));
    }
}
