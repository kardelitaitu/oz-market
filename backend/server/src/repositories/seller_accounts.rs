use crate::models::db::SellerAccountRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};

#[async_trait::async_trait]
pub trait SellerAccountRepository: Send + Sync {
    async fn get_by_owner_id(
        &self,
        owner_id: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError>;
}

pub fn not_found(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::NotFound, message)
}
