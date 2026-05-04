use crate::models::db::AuditEventRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};

#[async_trait::async_trait]
pub trait AuditEventRepository: Send + Sync {
    async fn append_event(&self, event: AuditEventRow) -> Result<(), RepositoryError>;
}

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}
