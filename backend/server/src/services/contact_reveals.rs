use crate::repositories::contact_reveals::InMemoryContactRevealRepository;
use crate::repositories::{ContactRevealRepository, RepositoryError};
use marketplace_api_contract::{ContactRevealResponse, RequestContactRevealRequest};
use std::sync::Arc;

pub struct ContactRevealService<R> {
    repository: Arc<R>,
}

impl<R> ContactRevealService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> ContactRevealService<R>
where
    R: ContactRevealRepository + Send + Sync,
{
    pub async fn create_request(
        &self,
        negotiation_id: &str,
        request: &RequestContactRevealRequest,
        buyer_agent_id: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, RepositoryError> {
        self.repository
            .as_ref()
            .create_request(negotiation_id, request, buyer_agent_id, now_rfc3339)
            .await
    }

    pub async fn approve_request(
        &self,
        reveal_id: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, RepositoryError> {
        self.repository
            .as_ref()
            .approve_request(reveal_id, now_rfc3339)
            .await
    }

    pub async fn get_by_reveal_id(
        &self,
        reveal_id: &str,
    ) -> Result<Option<ContactRevealResponse>, RepositoryError> {
        self.repository.as_ref().get_by_reveal_id(reveal_id).await
    }
}

pub type InMemoryContactRevealService = ContactRevealService<InMemoryContactRevealRepository>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::RepositoryErrorKind;

    #[tokio::test]
    async fn service_round_trips_reveal_state() {
        let service = ContactRevealService::new(Arc::new(InMemoryContactRevealRepository::new()));
        let created = service
            .create_request(
                "neg_1",
                &RequestContactRevealRequest {
                    idempotency_key: "idem-1".to_string(),
                },
                "buyer-1",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();
        assert_eq!(
            created.reveal_status,
            marketplace_api_contract::ContactRevealStatus::Pending
        );

        let approved = service
            .approve_request(&created.reveal_id, "2026-05-04T00:05:00Z")
            .await
            .unwrap();
        assert_eq!(
            approved.reveal_status,
            marketplace_api_contract::ContactRevealStatus::Approved
        );
    }

    #[tokio::test]
    async fn service_approve_nonexistent_reveal_returns_not_found() {
        let service = ContactRevealService::new(Arc::new(InMemoryContactRevealRepository::new()));
        let result = service
            .approve_request("rev_nonexistent", "2026-05-04T00:05:00Z")
            .await;
        assert!(matches!(
            result,
            Err(RepositoryError {
                kind: RepositoryErrorKind::NotFound,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn service_get_missing_reveal_returns_none() {
        let service = ContactRevealService::new(Arc::new(InMemoryContactRevealRepository::new()));
        let result = service.get_by_reveal_id("rev_nonexistent").await.unwrap();
        assert!(result.is_none());
    }
}
