use crate::repositories::{RepositoryError, RepositoryErrorKind};
use marketplace_api_contract::{NegotiationResponse, OpenNegotiationRequest, SubmitOfferRequest};

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
