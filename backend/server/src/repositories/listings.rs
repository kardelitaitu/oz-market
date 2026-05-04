use crate::repositories::{RepositoryError, RepositoryErrorKind};
use marketplace_api_contract::{
    CreateListingRequest, CreateListingResponse, ListingSummary, SearchRequest, SearchResponse,
};

#[async_trait::async_trait]
pub trait ListingRepository: Send + Sync {
    async fn insert_listing(
        &self,
        request: &CreateListingRequest,
    ) -> Result<CreateListingResponse, RepositoryError>;

    async fn get_listing(
        &self,
        listing_id: &str,
    ) -> Result<Option<ListingSummary>, RepositoryError>;

    async fn search_listings(
        &self,
        request: &SearchRequest,
    ) -> Result<SearchResponse, RepositoryError>;
}

pub fn conflict(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Conflict, message)
}
