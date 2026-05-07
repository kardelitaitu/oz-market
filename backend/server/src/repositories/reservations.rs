use crate::models::db::ReservationLeaseRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};
use async_trait::async_trait;
use sqlx::{
    postgres::{PgPool, PgRow},
    Row,
};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
};

#[async_trait]
pub trait ReservationLeaseRepository: Send + Sync {
    async fn reserve(
        &self,
        listing_id: &str,
        negotiation_id: &str,
        reserved_by: &str,
        now_rfc3339: &str,
        ttl_seconds: i64,
    ) -> Result<ReservationLeaseRow, RepositoryError>;

    async fn get_active_by_listing(
        &self,
        listing_id: &str,
    ) -> Result<Option<ReservationLeaseRow>, RepositoryError>;

    async fn release(
        &self,
        lease_id: &str,
        now_rfc3339: &str,
    ) -> Result<Option<ReservationLeaseRow>, RepositoryError>;
}

pub fn conflict(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Conflict, message)
}

pub fn not_found(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::NotFound, message)
}

pub fn validation(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Validation, message)
}

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}

pub struct InMemoryReservationLeaseRepository {
    by_lease_id: RwLock<HashMap<String, ReservationLeaseRow>>,
    by_listing_id: RwLock<HashMap<String, String>>,
    next_lease_id: AtomicU64,
}

impl InMemoryReservationLeaseRepository {
    pub fn new() -> Self {
        Self {
            by_lease_id: RwLock::new(HashMap::new()),
            by_listing_id: RwLock::new(HashMap::new()),
            next_lease_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> String {
        let next = self.next_lease_id.fetch_add(1, Ordering::SeqCst);
        format!("lease_{next:06}")
    }

    fn is_active(lease: &ReservationLeaseRow) -> bool {
        lease.status.eq_ignore_ascii_case("active")
    }
}

impl Default for InMemoryReservationLeaseRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReservationLeaseRepository for InMemoryReservationLeaseRepository {
    async fn reserve(
        &self,
        listing_id: &str,
        negotiation_id: &str,
        reserved_by: &str,
        now_rfc3339: &str,
        ttl_seconds: i64,
    ) -> Result<ReservationLeaseRow, RepositoryError> {
        if ttl_seconds <= 0 {
            return Err(RepositoryError::new(
                RepositoryErrorKind::Validation,
                "ttl_seconds must be positive",
            ));
        }

        let mut by_listing = self
            .by_listing_id
            .write()
            .expect("reservation listing write lock");
        let mut by_lease = self
            .by_lease_id
            .write()
            .expect("reservation lease write lock");

        if let Some(existing_lease_id) = by_listing.get(listing_id) {
            if let Some(existing) = by_lease.get(existing_lease_id) {
                if Self::is_active(existing) {
                    return Err(conflict("listing already has an active reservation lease"));
                }
            }
        }

        let lease = ReservationLeaseRow {
            lease_id: self.next_id(),
            negotiation_id: negotiation_id.to_string(),
            listing_id: listing_id.to_string(),
            reserved_by: reserved_by.to_string(),
            status: "active".to_string(),
            expires_at: format!("{now_rfc3339}+{ttl_seconds}s"),
            created_at: now_rfc3339.to_string(),
            updated_at: now_rfc3339.to_string(),
        };

        by_listing.insert(listing_id.to_string(), lease.lease_id.clone());
        by_lease.insert(lease.lease_id.clone(), lease.clone());
        Ok(lease)
    }

    async fn get_active_by_listing(
        &self,
        listing_id: &str,
    ) -> Result<Option<ReservationLeaseRow>, RepositoryError> {
        let by_listing = self
            .by_listing_id
            .read()
            .expect("reservation listing read lock");
        let Some(lease_id) = by_listing.get(listing_id) else {
            return Ok(None);
        };
        let by_lease = self
            .by_lease_id
            .read()
            .expect("reservation lease read lock");
        Ok(by_lease.get(lease_id).cloned().filter(Self::is_active))
    }

    async fn release(
        &self,
        lease_id: &str,
        now_rfc3339: &str,
    ) -> Result<Option<ReservationLeaseRow>, RepositoryError> {
        let mut by_lease = self
            .by_lease_id
            .write()
            .expect("reservation lease write lock");
        let mut by_listing = self
            .by_listing_id
            .write()
            .expect("reservation listing write lock");
        let Some(lease) = by_lease.get_mut(lease_id) else {
            return Ok(None);
        };

        lease.status = "cancelled".to_string();
        lease.updated_at = now_rfc3339.to_string();
        by_listing.remove(&lease.listing_id);
        Ok(Some(lease.clone()))
    }
}

pub struct PostgresReservationLeaseRepository {
    pool: Arc<PgPool>,
}

impl PostgresReservationLeaseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    fn row_to_lease(row: &PgRow) -> Result<ReservationLeaseRow, RepositoryError> {
        Ok(ReservationLeaseRow {
            lease_id: row
                .try_get("lease_id")
                .map_err(|error| storage(error.to_string()))?,
            negotiation_id: row
                .try_get("negotiation_id")
                .map_err(|error| storage(error.to_string()))?,
            listing_id: row
                .try_get("listing_id")
                .map_err(|error| storage(error.to_string()))?,
            reserved_by: row
                .try_get("reserved_by")
                .map_err(|error| storage(error.to_string()))?,
            status: row
                .try_get("status")
                .map_err(|error| storage(error.to_string()))?,
            expires_at: row
                .try_get("expires_at")
                .map_err(|error| storage(error.to_string()))?,
            created_at: row
                .try_get("created_at")
                .map_err(|error| storage(error.to_string()))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|error| storage(error.to_string()))?,
        })
    }

    async fn next_lease_id(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<String, RepositoryError> {
        let next = sqlx::query_scalar::<_, i64>("SELECT nextval('reservation_lease_id_seq')")
            .fetch_one(&mut **tx)
            .await
            .map_err(|error| storage(error.to_string()))?;
        Ok(format!("lease_{next:06}"))
    }
}

#[async_trait]
impl ReservationLeaseRepository for PostgresReservationLeaseRepository {
    async fn reserve(
        &self,
        listing_id: &str,
        negotiation_id: &str,
        reserved_by: &str,
        now_rfc3339: &str,
        ttl_seconds: i64,
    ) -> Result<ReservationLeaseRow, RepositoryError> {
        if ttl_seconds <= 0 {
            return Err(validation("ttl_seconds must be positive"));
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| storage(error.to_string()))?;

        let listing_row =
            sqlx::query("SELECT listing_id FROM listings WHERE listing_id = $1 FOR UPDATE")
                .bind(listing_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|error| storage(error.to_string()))?;
        if listing_row.is_none() {
            return Err(not_found("listing not found"));
        }

        let active_row = sqlx::query(
            "SELECT lease_id FROM reservation_leases WHERE listing_id = $1 AND status = 'active' FOR UPDATE",
        )
        .bind(listing_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| storage(error.to_string()))?;
        if active_row.is_some() {
            return Err(conflict("listing already has an active reservation lease"));
        }

        let lease_id = Self::next_lease_id(&mut tx).await?;
        let row = sqlx::query(
            "INSERT INTO reservation_leases (lease_id, negotiation_id, listing_id, reserved_by, status, expires_at, created_at, updated_at) VALUES ($1, $2, $3, $4, 'active', ($5::timestamptz + make_interval(secs => $6)), $5::timestamptz, $5::timestamptz) RETURNING lease_id, negotiation_id, listing_id, reserved_by, status, to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at",
        )
        .bind(&lease_id)
        .bind(negotiation_id)
        .bind(listing_id)
        .bind(reserved_by)
        .bind(now_rfc3339)
        .bind(ttl_seconds)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| storage(error.to_string()))?;

        tx.commit()
            .await
            .map_err(|error| storage(error.to_string()))?;
        Self::row_to_lease(&row)
    }

    async fn get_active_by_listing(
        &self,
        listing_id: &str,
    ) -> Result<Option<ReservationLeaseRow>, RepositoryError> {
        let row = sqlx::query(
            "SELECT lease_id, negotiation_id, listing_id, reserved_by, status, to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at FROM reservation_leases WHERE listing_id = $1 AND status = 'active' ORDER BY created_at DESC LIMIT 1",
        )
        .bind(listing_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|error| storage(error.to_string()))?;

        row.as_ref().map(Self::row_to_lease).transpose()
    }

    async fn release(
        &self,
        lease_id: &str,
        now_rfc3339: &str,
    ) -> Result<Option<ReservationLeaseRow>, RepositoryError> {
        let row = sqlx::query(
            "UPDATE reservation_leases SET status = 'cancelled', updated_at = $2::timestamptz WHERE lease_id = $1 RETURNING lease_id, negotiation_id, listing_id, reserved_by, status, to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at",
        )
        .bind(lease_id)
        .bind(now_rfc3339)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|error| storage(error.to_string()))?;

        row.as_ref().map(Self::row_to_lease).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reserve_blocks_double_sell() {
        let repo = InMemoryReservationLeaseRepository::new();
        let first = repo
            .reserve("lst_1", "neg_1", "buyer-1", "2026-05-04T00:00:00Z", 3600)
            .await
            .unwrap();
        assert_eq!(first.status, "active");
        let second = repo
            .reserve("lst_1", "neg_2", "buyer-2", "2026-05-04T00:00:01Z", 3600)
            .await;
        assert!(matches!(
            second,
            Err(RepositoryError {
                kind: RepositoryErrorKind::Conflict,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn release_allows_new_reservation() {
        let repo = InMemoryReservationLeaseRepository::new();
        let first = repo
            .reserve("lst_1", "neg_1", "buyer-1", "2026-05-04T00:00:00Z", 3600)
            .await
            .unwrap();
        let released = repo
            .release(&first.lease_id, "2026-05-04T01:00:00Z")
            .await
            .unwrap();
        assert!(released.is_some());
        let second = repo
            .reserve("lst_1", "neg_2", "buyer-2", "2026-05-04T01:00:01Z", 3600)
            .await
            .unwrap();
        assert_eq!(second.status, "active");
    }
}
