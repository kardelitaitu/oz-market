use crate::models::db::AuditEventRow;
use crate::repositories::{AuditEventRepository, RepositoryError};
use std::sync::Arc;

pub struct AuditEventService {
    repository: Arc<dyn AuditEventRepository>,
}

impl AuditEventService {
    pub fn new(repository: Arc<dyn AuditEventRepository>) -> Self {
        Self { repository }
    }

    pub async fn append_event(&self, event: AuditEventRow) -> Result<(), RepositoryError> {
        self.repository.append_event(event).await
    }
}
