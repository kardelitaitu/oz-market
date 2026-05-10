use crate::auth::AuthzError;
use crate::repositories::{ListingRepository, RepositoryError};
use crate::services::idempotency::{
    IdempotencyAttempt, IdempotencyDecision, IdempotencyGuard, IdempotencyOperation,
};
use crate::services::search::SearchService;
use marketplace_api_contract::{
    CreateListingRequest, ListingSummary, SearchRequest, SearchResponse,
};
use marketplace_auth_core::Claims;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum HandlerError {
    Authz(AuthzError),
    Idempotency(crate::services::idempotency::IdempotencyError),
    Search(crate::services::search::SearchError),
    Repository(RepositoryError),
    QuotaExceeded { message: String },
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authz(error) => write!(f, "{error}"),
            Self::Idempotency(error) => write!(f, "{error:?}"),
            Self::Search(error) => write!(f, "{error:?}"),
            Self::Repository(error) => write!(f, "{error}"),
            Self::QuotaExceeded { message } => write!(f, "quota exceeded: {message}"),
        }
    }
}

impl std::error::Error for HandlerError {}

impl From<AuthzError> for HandlerError {
    fn from(value: AuthzError) -> Self {
        Self::Authz(value)
    }
}

impl From<crate::services::idempotency::IdempotencyError> for HandlerError {
    fn from(value: crate::services::idempotency::IdempotencyError) -> Self {
        Self::Idempotency(value)
    }
}

impl From<crate::services::search::SearchError> for HandlerError {
    fn from(value: crate::services::search::SearchError) -> Self {
        Self::Search(value)
    }
}

impl From<RepositoryError> for HandlerError {
    fn from(value: RepositoryError) -> Self {
        Self::Repository(value)
    }
}

pub async fn search_listings<R>(
    service: &SearchService<R>,
    claims: Option<&Claims>,
    request: &SearchRequest,
) -> Result<SearchResponse, HandlerError>
where
    R: ListingRepository + Send + Sync,
{
    Ok(service.search_listings(claims, request).await?)
}

pub async fn get_listing<R>(
    service: &SearchService<R>,
    claims: Option<&Claims>,
    listing_id: &str,
) -> Result<Option<ListingSummary>, HandlerError>
where
    R: ListingRepository + Send + Sync,
{
    Ok(service.get_listing(claims, listing_id).await?)
}

pub async fn begin_create_listing<R>(
    guard: &IdempotencyGuard<R>,
    claims: &Claims,
    request: &CreateListingRequest,
    request_fingerprint: &str,
    now_rfc3339: &str,
) -> Result<IdempotencyDecision, HandlerError>
where
    R: crate::repositories::IdempotencyKeyRepository + Send + Sync,
{
    crate::auth::authorize_create_listing(claims, &request.listing.owner_id)?;
    Ok(guard
        .begin(
            &IdempotencyAttempt {
                actor_subject: &claims.sub,
                operation: IdempotencyOperation::CreateListing,
                idempotency_key: &request.idempotency_key,
                request_fingerprint,
                ttl_seconds: 24 * 60 * 60,
            },
            now_rfc3339,
        )
        .await?)
}

pub async fn begin_open_negotiation<R>(
    guard: &IdempotencyGuard<R>,
    claims: &Claims,
    request: &marketplace_api_contract::OpenNegotiationRequest,
    request_fingerprint: &str,
    now_rfc3339: &str,
) -> Result<IdempotencyDecision, HandlerError>
where
    R: crate::repositories::IdempotencyKeyRepository + Send + Sync,
{
    crate::auth::authorize_open_negotiation(claims, &request.buyer_agent_id)?;
    Ok(guard
        .begin(
            &IdempotencyAttempt {
                actor_subject: &claims.sub,
                operation: IdempotencyOperation::OpenNegotiation,
                idempotency_key: &request.idempotency_key,
                request_fingerprint,
                ttl_seconds: 24 * 60 * 60,
            },
            now_rfc3339,
        )
        .await?)
}
