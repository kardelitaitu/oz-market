use crate::models::db::OutboxEventRow;
use crate::repositories::{OutboxEventRepository, RepositoryError};
use std::sync::Arc;

pub struct OutboxEventService {
    repository: Arc<dyn OutboxEventRepository>,
}

impl OutboxEventService {
    pub fn new(repository: Arc<dyn OutboxEventRepository>) -> Self {
        Self { repository }
    }

    pub async fn append_event(&self, event: OutboxEventRow) -> Result<(), RepositoryError> {
        self.repository.append_event(event).await
    }
}
