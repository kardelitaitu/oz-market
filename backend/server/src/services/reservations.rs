use crate::models::db::ReservationLeaseRow;
use crate::repositories::reservations::InMemoryReservationLeaseRepository;
use crate::repositories::{RepositoryError, ReservationLeaseRepository};
use std::sync::Arc;

pub struct ReservationLeaseService<R> {
    repository: Arc<R>,
}

impl<R> ReservationLeaseService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> ReservationLeaseService<R>
where
    R: ReservationLeaseRepository + Send + Sync,
{
    pub async fn reserve(
        &self,
        listing_id: &str,
        negotiation_id: &str,
        reserved_by: &str,
        now_rfc3339: &str,
        ttl_seconds: i64,
    ) -> Result<ReservationLeaseRow, RepositoryError> {
        self.repository
            .as_ref()
            .reserve(
                listing_id,
                negotiation_id,
                reserved_by,
                now_rfc3339,
                ttl_seconds,
            )
            .await
    }

    pub async fn get_active_by_listing(
        &self,
        listing_id: &str,
    ) -> Result<Option<ReservationLeaseRow>, RepositoryError> {
        self.repository
            .as_ref()
            .get_active_by_listing(listing_id)
            .await
    }

    pub async fn release(
        &self,
        lease_id: &str,
        now_rfc3339: &str,
    ) -> Result<Option<ReservationLeaseRow>, RepositoryError> {
        self.repository
            .as_ref()
            .release(lease_id, now_rfc3339)
            .await
    }
}

pub type InMemoryReservationLeaseService =
    ReservationLeaseService<InMemoryReservationLeaseRepository>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn service_blocks_double_sell() {
        let service =
            ReservationLeaseService::new(Arc::new(InMemoryReservationLeaseRepository::new()));
        let first = service
            .reserve("lst_1", "neg_1", "buyer-1", "2026-05-04T00:00:00Z", 3600)
            .await
            .unwrap();
        let second = service
            .reserve("lst_1", "neg_2", "buyer-2", "2026-05-04T00:00:01Z", 3600)
            .await;
        assert!(second.is_err());
        let active = service.get_active_by_listing("lst_1").await.unwrap();
        assert_eq!(active.unwrap().lease_id, first.lease_id);
    }
}
