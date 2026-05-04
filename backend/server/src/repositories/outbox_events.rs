use crate::models::db::OutboxEventRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};

#[async_trait::async_trait]
pub trait OutboxEventRepository: Send + Sync {
    async fn append_event(&self, event: OutboxEventRow) -> Result<(), RepositoryError>;
}

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}
