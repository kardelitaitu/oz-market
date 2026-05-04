use crate::repositories::{RepositoryError, RepositoryErrorKind};
use marketplace_api_contract::{ContactRevealResponse, RequestContactRevealRequest};

#[async_trait::async_trait]
pub trait ContactRevealRepository: Send + Sync {
    async fn create_request(
        &self,
        negotiation_id: &str,
        request: &RequestContactRevealRequest,
    ) -> Result<ContactRevealResponse, RepositoryError>;

    async fn approve_request(
        &self,
        reveal_id: &str,
    ) -> Result<ContactRevealResponse, RepositoryError>;
}

pub fn conflict(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Conflict, message)
}
