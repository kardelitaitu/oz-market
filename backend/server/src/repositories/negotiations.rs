use crate::repositories::{RepositoryError, RepositoryErrorKind};
use async_trait::async_trait;
use marketplace_api_contract::{NegotiationResponse, OpenNegotiationRequest, SubmitOfferRequest};
use std::sync::Arc;

#[async_trait::async_trait]
pub trait NegotiationRepository: Send + Sync {
    async fn open_negotiation(
        &self,
        request: &OpenNegotiationRequest,
    ) -> Result<NegotiationResponse, RepositoryError>;

    async fn get_negotiation(
        &self,
        negotiation_id: &str,
    ) -> Result<Option<NegotiationResponse>, RepositoryError>;

    async fn submit_offer(
        &self,
        negotiation_id: &str,
        request: &SubmitOfferRequest,
    ) -> Result<NegotiationResponse, RepositoryError>;
}

pub fn conflict(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Conflict, message)
}

pub struct InMemoryNegotiationRepository;

impl InMemoryNegotiationRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NegotiationRepository for InMemoryNegotiationRepository {
    async fn open_negotiation(
        &self,
        _request: &OpenNegotiationRequest,
    ) -> Result<NegotiationResponse, RepositoryError> {
        Err(RepositoryError::new(
            RepositoryErrorKind::NotFound,
            "InMemoryNegotiationRepository is a stub - negotiation data stored via reservation_leases",
        ))
    }

    async fn get_negotiation(
        &self,
        _negotiation_id: &str,
    ) -> Result<Option<NegotiationResponse>, RepositoryError> {
        Ok(None)
    }

    async fn submit_offer(
        &self,
        _negotiation_id: &str,
        _request: &SubmitOfferRequest,
    ) -> Result<NegotiationResponse, RepositoryError> {
        Err(RepositoryError::new(
            RepositoryErrorKind::NotFound,
            "InMemoryNegotiationRepository is a stub",
        ))
    }
}

pub struct PostgresNegotiationRepository {
    // This is a stub - pool is not needed since all methods return errors immediately
}

impl PostgresNegotiationRepository {
    pub fn new(_pool: Arc<sqlx::postgres::PgPool>) -> Self {
        Self {}
    }
}

#[async_trait]
impl NegotiationRepository for PostgresNegotiationRepository {
    async fn open_negotiation(
        &self,
        _request: &OpenNegotiationRequest,
    ) -> Result<NegotiationResponse, RepositoryError> {
        // Negotiations are tracked via reservation_leases table
        // This stub exists to satisfy the trait requirement
        Err(RepositoryError::new(
            RepositoryErrorKind::NotFound,
            "PostgresNegotiationRepository is a stub - negotiation data stored via reservation_leases",
        ))
    }

    async fn get_negotiation(
        &self,
        _negotiation_id: &str,
    ) -> Result<Option<NegotiationResponse>, RepositoryError> {
        Ok(None)
    }

    async fn submit_offer(
        &self,
        _negotiation_id: &str,
        _request: &SubmitOfferRequest,
    ) -> Result<NegotiationResponse, RepositoryError> {
        Err(RepositoryError::new(
            RepositoryErrorKind::NotFound,
            "PostgresNegotiationRepository is a stub",
        ))
    }
}
