use crate::repositories::{RepositoryError, RepositoryErrorKind};
use async_trait::async_trait;
use marketplace_api_contract::{
    AcceptNegotiationRequest, NegotiationHistoryEntry, NegotiationHistoryEntryType,
    NegotiationResponse, NegotiationStatus, RejectNegotiationRequest, SubmitOfferRequest,
};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[async_trait]
pub trait NegotiationRepository: Send + Sync {
    async fn upsert_open_negotiation(
        &self,
        response: &NegotiationResponse,
        open_idempotency_key: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError>;

    async fn get_negotiation(
        &self,
        negotiation_id: &str,
    ) -> Result<Option<NegotiationResponse>, RepositoryError>;

    async fn submit_offer(
        &self,
        negotiation_id: &str,
        request: &SubmitOfferRequest,
        actor_subject: &str,
        actor_role: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError>;

    async fn accept_negotiation(
        &self,
        negotiation_id: &str,
        request: &AcceptNegotiationRequest,
        actor_subject: &str,
        actor_role: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError>;

    async fn reject_negotiation(
        &self,
        negotiation_id: &str,
        request: &RejectNegotiationRequest,
        actor_subject: &str,
        actor_role: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError>;
}

pub fn conflict(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Conflict, message)
}

pub fn not_found(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::NotFound, message)
}

fn status_to_db(status: NegotiationStatus) -> &'static str {
    match status {
        NegotiationStatus::Open => "open",
        NegotiationStatus::Countered => "countered",
        NegotiationStatus::NearClose => "near_close",
        NegotiationStatus::Reserved => "reserved",
        NegotiationStatus::ContactRequested => "contact_requested",
        NegotiationStatus::ContactRevealed => "contact_revealed",
        NegotiationStatus::Closed => "closed",
        NegotiationStatus::Cancelled => "cancelled",
    }
}

fn status_from_db(status: &str) -> Result<NegotiationStatus, RepositoryError> {
    match status {
        "open" => Ok(NegotiationStatus::Open),
        "countered" => Ok(NegotiationStatus::Countered),
        "near_close" => Ok(NegotiationStatus::NearClose),
        "reserved" => Ok(NegotiationStatus::Reserved),
        "contact_requested" => Ok(NegotiationStatus::ContactRequested),
        "contact_revealed" => Ok(NegotiationStatus::ContactRevealed),
        "closed" => Ok(NegotiationStatus::Closed),
        "cancelled" => Ok(NegotiationStatus::Cancelled),
        other => Err(RepositoryError::new(
            RepositoryErrorKind::Storage,
            format!("unknown negotiation status: {other}"),
        )),
    }
}

fn next_entry_id(
    negotiation_id: &str,
    history_len: usize,
    entry_type: NegotiationHistoryEntryType,
) -> String {
    let suffix = match entry_type {
        NegotiationHistoryEntryType::Offer => "offer",
        NegotiationHistoryEntryType::Accept => "accept",
        NegotiationHistoryEntryType::Reject => "reject",
    };
    format!("{negotiation_id}-{suffix}-{}", history_len + 1)
}

fn update_offer_state(
    current: &mut NegotiationResponse,
    request: &SubmitOfferRequest,
    actor_subject: &str,
    actor_role: &str,
    now_rfc3339: &str,
) {
    current.status = NegotiationStatus::Countered;
    current.offer_currency = request.offer_currency.clone();
    current.latest_offer_amount = request.offer_amount;
    current.version += 1;
    current.updated_at = now_rfc3339.to_string();
    let entry_id = next_entry_id(
        &current.negotiation_id,
        current.offer_history.len(),
        NegotiationHistoryEntryType::Offer,
    );
    current.offer_history.push(NegotiationHistoryEntry {
        entry_id,
        entry_type: NegotiationHistoryEntryType::Offer,
        offer_currency: request.offer_currency.clone(),
        offer_amount: request.offer_amount,
        actor_subject: actor_subject.to_string(),
        actor_role: actor_role.to_string(),
        idempotency_key: request.idempotency_key.clone(),
        resulting_status: current.status,
        created_at: now_rfc3339.to_string(),
    });
}

fn update_accept_state(
    current: &mut NegotiationResponse,
    request: &AcceptNegotiationRequest,
    actor_subject: &str,
    actor_role: &str,
    now_rfc3339: &str,
) {
    current.status = NegotiationStatus::Closed;
    current.final_offer_amount = Some(current.latest_offer_amount);
    current.version += 1;
    current.updated_at = now_rfc3339.to_string();
    let entry_id = next_entry_id(
        &current.negotiation_id,
        current.offer_history.len(),
        NegotiationHistoryEntryType::Accept,
    );
    current.offer_history.push(NegotiationHistoryEntry {
        entry_id,
        entry_type: NegotiationHistoryEntryType::Accept,
        offer_currency: current.offer_currency.clone(),
        offer_amount: current.latest_offer_amount,
        actor_subject: actor_subject.to_string(),
        actor_role: actor_role.to_string(),
        idempotency_key: request.idempotency_key.clone(),
        resulting_status: current.status,
        created_at: now_rfc3339.to_string(),
    });
}

fn update_reject_state(
    current: &mut NegotiationResponse,
    request: &RejectNegotiationRequest,
    actor_subject: &str,
    actor_role: &str,
    now_rfc3339: &str,
) {
    current.status = NegotiationStatus::Cancelled;
    current.version += 1;
    current.updated_at = now_rfc3339.to_string();
    let entry_id = next_entry_id(
        &current.negotiation_id,
        current.offer_history.len(),
        NegotiationHistoryEntryType::Reject,
    );
    current.offer_history.push(NegotiationHistoryEntry {
        entry_id,
        entry_type: NegotiationHistoryEntryType::Reject,
        offer_currency: current.offer_currency.clone(),
        offer_amount: current.latest_offer_amount,
        actor_subject: actor_subject.to_string(),
        actor_role: actor_role.to_string(),
        idempotency_key: request.idempotency_key.clone(),
        resulting_status: current.status,
        created_at: now_rfc3339.to_string(),
    });
}

fn parse_history(
    value: serde_json::Value,
) -> Result<Vec<NegotiationHistoryEntry>, RepositoryError> {
    serde_json::from_value(value).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorKind::Storage,
            format!("invalid offer_history payload: {error}"),
        )
    })
}

#[derive(Default)]
pub struct InMemoryNegotiationRepository {
    negotiations: RwLock<HashMap<String, NegotiationResponse>>,
}

impl InMemoryNegotiationRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl NegotiationRepository for InMemoryNegotiationRepository {
    async fn upsert_open_negotiation(
        &self,
        response: &NegotiationResponse,
        _open_idempotency_key: &str,
        _now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError> {
        let mut write = self
            .negotiations
            .write()
            .expect("negotiation in-memory write lock");
        if write.contains_key(&response.negotiation_id) {
            return Err(conflict("negotiation already exists"));
        }
        write.insert(response.negotiation_id.clone(), response.clone());
        Ok(response.clone())
    }

    async fn get_negotiation(
        &self,
        negotiation_id: &str,
    ) -> Result<Option<NegotiationResponse>, RepositoryError> {
        let read = self
            .negotiations
            .read()
            .expect("negotiation in-memory read lock");
        Ok(read.get(negotiation_id).cloned())
    }

    async fn submit_offer(
        &self,
        negotiation_id: &str,
        request: &SubmitOfferRequest,
        actor_subject: &str,
        actor_role: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError> {
        let mut write = self
            .negotiations
            .write()
            .expect("negotiation in-memory write lock");
        let current = write
            .get_mut(negotiation_id)
            .ok_or_else(|| not_found("negotiation not found"))?;
        update_offer_state(current, request, actor_subject, actor_role, now_rfc3339);
        Ok(current.clone())
    }

    async fn accept_negotiation(
        &self,
        negotiation_id: &str,
        request: &AcceptNegotiationRequest,
        actor_subject: &str,
        actor_role: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError> {
        let mut write = self
            .negotiations
            .write()
            .expect("negotiation in-memory write lock");
        let current = write
            .get_mut(negotiation_id)
            .ok_or_else(|| not_found("negotiation not found"))?;
        update_accept_state(current, request, actor_subject, actor_role, now_rfc3339);
        Ok(current.clone())
    }

    async fn reject_negotiation(
        &self,
        negotiation_id: &str,
        request: &RejectNegotiationRequest,
        actor_subject: &str,
        actor_role: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError> {
        let mut write = self
            .negotiations
            .write()
            .expect("negotiation in-memory write lock");
        let current = write
            .get_mut(negotiation_id)
            .ok_or_else(|| not_found("negotiation not found"))?;
        update_reject_state(current, request, actor_subject, actor_role, now_rfc3339);
        Ok(current.clone())
    }
}

pub struct PostgresNegotiationRepository {
    pool: Arc<sqlx::postgres::PgPool>,
}

impl PostgresNegotiationRepository {
    pub fn new(pool: sqlx::postgres::PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    fn row_to_response(
        row: &sqlx::postgres::PgRow,
    ) -> Result<NegotiationResponse, RepositoryError> {
        let status_text: String = row
            .try_get("status")
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let status = status_from_db(&status_text)?;
        let offer_history_value: serde_json::Value = row
            .try_get("offer_history")
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let offer_history = parse_history(offer_history_value)?;

        Ok(NegotiationResponse {
            negotiation_id: row
                .try_get("negotiation_id")
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?,
            listing_id: row
                .try_get("listing_id")
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?,
            buyer_agent_id: row
                .try_get("buyer_agent_id")
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?,
            status,
            offer_currency: row
                .try_get("offer_currency")
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?,
            latest_offer_amount: row
                .try_get("latest_offer_amount")
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?,
            reservation_lease_id: row
                .try_get("reservation_lease_id")
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?,
            final_offer_amount: row
                .try_get("final_offer_amount")
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?,
            offer_history,
            version: row
                .try_get::<i64, _>("version")
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?
                as u64,
            updated_at: row
                .try_get("updated_at")
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?,
        })
    }
}

#[async_trait]
impl NegotiationRepository for PostgresNegotiationRepository {
    async fn upsert_open_negotiation(
        &self,
        response: &NegotiationResponse,
        open_idempotency_key: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError> {
        let offer_history = serde_json::to_value(&response.offer_history)
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let result = sqlx::query(
            "INSERT INTO negotiations (
                negotiation_id, listing_id, buyer_agent_id, status, offer_currency,
                latest_offer_amount, reservation_lease_id, final_offer_amount, offer_history,
                version, open_idempotency_key, created_at, updated_at
              ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::timestamptz, $12::timestamptz)
             ON CONFLICT (negotiation_id) DO NOTHING",
        )
        .bind(&response.negotiation_id)
        .bind(&response.listing_id)
        .bind(&response.buyer_agent_id)
        .bind(status_to_db(response.status))
        .bind(&response.offer_currency)
        .bind(response.latest_offer_amount)
        .bind(&response.reservation_lease_id)
        .bind(response.final_offer_amount)
        .bind(offer_history)
        .bind(response.version as i64)
        .bind(open_idempotency_key)
        .bind(now_rfc3339)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(conflict("negotiation already exists"));
        }

        Ok(response.clone())
    }

    async fn get_negotiation(
        &self,
        negotiation_id: &str,
    ) -> Result<Option<NegotiationResponse>, RepositoryError> {
        let row = sqlx::query(
            "SELECT
                negotiation_id,
                listing_id,
                buyer_agent_id,
                status,
                offer_currency,
                latest_offer_amount,
                reservation_lease_id,
                final_offer_amount,
                offer_history,
                version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at
             FROM negotiations
             WHERE negotiation_id = $1",
        )
        .bind(negotiation_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        row.map(|r| Self::row_to_response(&r)).transpose()
    }

    async fn submit_offer(
        &self,
        negotiation_id: &str,
        request: &SubmitOfferRequest,
        actor_subject: &str,
        actor_role: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let row = sqlx::query(
            "SELECT
                negotiation_id,
                listing_id,
                buyer_agent_id,
                status,
                offer_currency,
                latest_offer_amount,
                reservation_lease_id,
                final_offer_amount,
                offer_history,
                version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at
             FROM negotiations
             WHERE negotiation_id = $1
             FOR UPDATE",
        )
        .bind(negotiation_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?
        .ok_or_else(|| not_found("negotiation not found"))?;

        let mut response = Self::row_to_response(&row)?;
        update_offer_state(
            &mut response,
            request,
            actor_subject,
            actor_role,
            now_rfc3339,
        );
        let history = serde_json::to_value(&response.offer_history)
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        sqlx::query(
            "UPDATE negotiations
             SET status = $2,
                 offer_currency = $3,
                 latest_offer_amount = $4,
                 final_offer_amount = $5,
                 offer_history = $6,
                 version = $7,
                 updated_at = $8::timestamptz
             WHERE negotiation_id = $1",
        )
        .bind(negotiation_id)
        .bind(status_to_db(response.status))
        .bind(&response.offer_currency)
        .bind(response.latest_offer_amount)
        .bind(response.final_offer_amount)
        .bind(history)
        .bind(response.version as i64)
        .bind(now_rfc3339)
        .execute(tx.as_mut())
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(response)
    }

    async fn accept_negotiation(
        &self,
        negotiation_id: &str,
        request: &AcceptNegotiationRequest,
        actor_subject: &str,
        actor_role: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let row = sqlx::query(
            "SELECT
                negotiation_id,
                listing_id,
                buyer_agent_id,
                status,
                offer_currency,
                latest_offer_amount,
                reservation_lease_id,
                final_offer_amount,
                offer_history,
                version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at
             FROM negotiations
             WHERE negotiation_id = $1
             FOR UPDATE",
        )
        .bind(negotiation_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?
        .ok_or_else(|| not_found("negotiation not found"))?;

        let mut response = Self::row_to_response(&row)?;
        update_accept_state(
            &mut response,
            request,
            actor_subject,
            actor_role,
            now_rfc3339,
        );
        let history = serde_json::to_value(&response.offer_history)
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        sqlx::query(
            "UPDATE negotiations
             SET status = $2,
                 final_offer_amount = $3,
                 offer_history = $4,
                 version = $5,
                 updated_at = $6::timestamptz
             WHERE negotiation_id = $1",
        )
        .bind(negotiation_id)
        .bind(status_to_db(response.status))
        .bind(response.final_offer_amount)
        .bind(history)
        .bind(response.version as i64)
        .bind(now_rfc3339)
        .execute(tx.as_mut())
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(response)
    }

    async fn reject_negotiation(
        &self,
        negotiation_id: &str,
        request: &RejectNegotiationRequest,
        actor_subject: &str,
        actor_role: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let row = sqlx::query(
            "SELECT
                negotiation_id,
                listing_id,
                buyer_agent_id,
                status,
                offer_currency,
                latest_offer_amount,
                reservation_lease_id,
                final_offer_amount,
                offer_history,
                version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at
             FROM negotiations
             WHERE negotiation_id = $1
             FOR UPDATE",
        )
        .bind(negotiation_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?
        .ok_or_else(|| not_found("negotiation not found"))?;

        let mut response = Self::row_to_response(&row)?;
        update_reject_state(
            &mut response,
            request,
            actor_subject,
            actor_role,
            now_rfc3339,
        );
        let history = serde_json::to_value(&response.offer_history)
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        sqlx::query(
            "UPDATE negotiations
             SET status = $2,
                 offer_history = $3,
                 version = $4,
                 updated_at = $5::timestamptz
             WHERE negotiation_id = $1",
        )
        .bind(negotiation_id)
        .bind(status_to_db(response.status))
        .bind(history)
        .bind(response.version as i64)
        .bind(now_rfc3339)
        .execute(tx.as_mut())
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(response)
    }
}
