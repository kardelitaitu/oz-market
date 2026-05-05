use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Missing or malformed authorization header")]
    MissingAuthHeader,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    ExpiredToken,

    #[error("Insufficient permissions: required scope {0:?}")]
    InsufficientScope(super::Scope),

    #[error("Insufficient role: required role {0:?}")]
    InsufficientRole(super::Role),

    #[error("Ownership check failed")]
    OwnershipFailed,
}

pub type AuthResult<T> = Result<T, AuthError>;
