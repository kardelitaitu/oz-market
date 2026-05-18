use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    InvalidField,
    MissingField,
    Conflict,
    NotFound,
    RateLimited,
    Unauthorized,
    Forbidden,
    OwnerMismatch,
    CredentialRevoked,
    QuotaExceeded,
    TrustReviewRequired,
    ReservationConflict,
    VersionConflict,
    InvalidTransition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiErrorDetail {
    pub code: ApiErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiErrorResponse {
    pub error: ApiErrorDetail,
}
