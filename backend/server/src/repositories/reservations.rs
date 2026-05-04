use crate::repositories::{RepositoryError, RepositoryErrorKind};

#[async_trait::async_trait]
pub trait ReservationLeaseRepository: Send + Sync {
    async fn create_for_negotiation(
        &self,
        negotiation_id: &str,
    ) -> Result<String, RepositoryError>;

    async fn get_active_by_listing(
        &self,
        listing_id: &str,
    ) -> Result<Option<String>, RepositoryError>;
}

pub fn conflict(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Conflict, message)
}
