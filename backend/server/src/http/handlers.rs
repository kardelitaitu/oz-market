use crate::auth::AuthzError;
use crate::domain::listing_validation::ValidationError as DomainValidationError;
use crate::repositories::{ListingRepository, RepositoryError};
use crate::services::agent::AgentError;
use crate::services::idempotency::{
    IdempotencyAttempt, IdempotencyDecision, IdempotencyGuard, IdempotencyOperation,
};
use crate::services::search::SearchService;
use marketplace_api_contract::{
    ApiErrorCode, ApiErrorDetail, ApiErrorResponse, CreateListingRequest, ListingSummary,
    SearchRequest, SearchResponse,
};
use marketplace_auth_core::Claims;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum HandlerError {
    Agent(AgentError),
    Authz(AuthzError),
    Idempotency(crate::services::idempotency::IdempotencyError),
    Search(crate::services::search::SearchError),
    Repository(RepositoryError),
    QuotaExceeded { message: String },
    Validation { field: String, message: String },
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Agent(error) => write!(f, "{error}"),
            Self::Authz(error) => write!(f, "{error}"),
            Self::Idempotency(error) => write!(f, "{error:?}"),
            Self::Search(error) => write!(f, "{error:?}"),
            Self::Repository(error) => write!(f, "{error}"),
            Self::QuotaExceeded { message } => write!(f, "quota exceeded: {message}"),
            Self::Validation { field, message } => {
                write!(f, "validation error on {field}: {message}")
            }
        }
    }
}

impl std::error::Error for HandlerError {}

impl HandlerError {
    pub fn to_http_parts(&self) -> (u16, ApiErrorResponse) {
        let (status, code, field) = match self {
            Self::Authz(inner) => match inner.kind {
                crate::auth::AuthzErrorKind::MissingScope
                | crate::auth::AuthzErrorKind::MissingRole => (403, ApiErrorCode::Forbidden, None),
                crate::auth::AuthzErrorKind::OwnershipMismatch => {
                    (403, ApiErrorCode::OwnerMismatch, Some("owner_id".into()))
                }
            },
            Self::Idempotency(inner) => match inner.kind {
                crate::services::idempotency::IdempotencyErrorKind::InvalidKey => (
                    400,
                    ApiErrorCode::InvalidField,
                    Some("idempotency_key".into()),
                ),
                crate::services::idempotency::IdempotencyErrorKind::Conflict => {
                    (409, ApiErrorCode::VersionConflict, None)
                }
                crate::services::idempotency::IdempotencyErrorKind::Storage => {
                    (500, ApiErrorCode::Conflict, None)
                }
            },
            Self::Search(inner) => match inner {
                crate::services::search::SearchError::Authz(authz) => match authz.kind {
                    crate::auth::AuthzErrorKind::MissingScope
                    | crate::auth::AuthzErrorKind::MissingRole => {
                        (403, ApiErrorCode::Forbidden, None)
                    }
                    crate::auth::AuthzErrorKind::OwnershipMismatch => {
                        (403, ApiErrorCode::OwnerMismatch, None)
                    }
                },
                crate::services::search::SearchError::Storage(_) => {
                    (500, ApiErrorCode::Conflict, None)
                }
            },
            Self::Repository(repo) => match repo.kind {
                crate::repositories::RepositoryErrorKind::Conflict => {
                    (409, ApiErrorCode::Conflict, None)
                }
                crate::repositories::RepositoryErrorKind::NotFound => {
                    (404, ApiErrorCode::NotFound, None)
                }
                crate::repositories::RepositoryErrorKind::PermissionDenied => {
                    (403, ApiErrorCode::Forbidden, None)
                }
                crate::repositories::RepositoryErrorKind::Validation => {
                    (400, ApiErrorCode::InvalidField, None)
                }
                crate::repositories::RepositoryErrorKind::Storage
                | crate::repositories::RepositoryErrorKind::Unknown => {
                    (500, ApiErrorCode::Conflict, None)
                }
            },
            Self::QuotaExceeded { .. } => (429, ApiErrorCode::QuotaExceeded, None),
            Self::Agent(_) => (400, ApiErrorCode::InvalidField, None),
            Self::Validation { field, .. } => {
                (400, ApiErrorCode::InvalidField, Some(field.clone()))
            }
        };
        let message = self.to_string();
        (
            status,
            ApiErrorResponse {
                error: ApiErrorDetail {
                    code,
                    message,
                    field,
                },
            },
        )
    }
}

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

impl From<DomainValidationError> for HandlerError {
    fn from(value: DomainValidationError) -> Self {
        Self::Validation {
            field: value.field,
            message: value.message,
        }
    }
}

impl From<Vec<DomainValidationError>> for HandlerError {
    fn from(errors: Vec<DomainValidationError>) -> Self {
        // Take the first validation error as the primary error
        if let Some(first) = errors.into_iter().next() {
            Self::Validation {
                field: first.field,
                message: first.message,
            }
        } else {
            Self::Validation {
                field: "unknown".into(),
                message: "validation failed".into(),
            }
        }
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
