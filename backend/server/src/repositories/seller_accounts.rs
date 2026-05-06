use crate::models::db::SellerAccountRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::RwLock;

#[async_trait::async_trait]
pub trait SellerAccountRepository: Send + Sync {
    async fn get_by_owner_id(
        &self,
        owner_id: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError>;

    async fn update_trust_level(
        &self,
        seller_account_id: &str,
        trust_level: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError>;

    async fn update_quota_override(
        &self,
        seller_account_id: &str,
        quota_override: Option<i32>,
    ) -> Result<Option<SellerAccountRow>, RepositoryError>;

    async fn increment_listings_created(
        &self,
        seller_account_id: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError>;
}

pub fn not_found(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::NotFound, message)
}

// InMemory Implementation
pub struct InMemorySellerAccountRepository {
    accounts: RwLock<HashMap<String, SellerAccountRow>>,
}

impl InMemorySellerAccountRepository {
    pub fn new() -> Self {
        Self {
            accounts: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_account(&self, account: SellerAccountRow) {
        let mut guard = self.accounts.write().expect("seller account write lock");
        guard.insert(account.seller_account_id.clone(), account);
    }
}

#[async_trait::async_trait]
impl SellerAccountRepository for InMemorySellerAccountRepository {
    async fn get_by_owner_id(
        &self,
        owner_id: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError> {
        let guard = self.accounts.read().expect("seller account read lock");
        Ok(guard
            .values()
            .find(|account| account.owner_id == owner_id)
            .cloned())
    }

    async fn update_trust_level(
        &self,
        seller_account_id: &str,
        trust_level: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError> {
        let mut guard = self.accounts.write().expect("seller account write lock");
        if let Some(account) = guard.get_mut(seller_account_id) {
            account.trust_level = trust_level.to_string();
            Ok(Some(account.clone()))
        } else {
            Ok(None)
        }
    }

    async fn update_quota_override(
        &self,
        seller_account_id: &str,
        quota_override: Option<i32>,
    ) -> Result<Option<SellerAccountRow>, RepositoryError> {
        let mut guard = self.accounts.write().expect("seller account write lock");
        if let Some(account) = guard.get_mut(seller_account_id) {
            account.quota_override = quota_override;
            Ok(Some(account.clone()))
        } else {
            Ok(None)
        }
    }

    async fn increment_listings_created(
        &self,
        seller_account_id: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError> {
        let mut guard = self.accounts.write().expect("seller account write lock");
        if let Some(account) = guard.get_mut(seller_account_id) {
            account.listings_created += 1;
            Ok(Some(account.clone()))
        } else {
            Ok(None)
        }
    }
}

// Postgres Implementation
pub struct PostgresSellerAccountRepository {
    pool: sqlx::postgres::PgPool,
}

impl PostgresSellerAccountRepository {
    pub fn new(pool: sqlx::postgres::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl SellerAccountRepository for PostgresSellerAccountRepository {
    async fn get_by_owner_id(
        &self,
        owner_id: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let row = sqlx::query(
            "SELECT seller_account_id, owner_id, trust_level, quota_override, listings_created, status, hardware_fingerprint, verified_at, created_at, updated_at FROM seller_accounts WHERE owner_id = $1",
        )
        .bind(owner_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(row.map(|r| SellerAccountRow {
            seller_account_id: r.get("seller_account_id"),
            owner_id: r.get("owner_id"),
            trust_level: r.get("trust_level"),
            quota_override: r.get("quota_override"),
            listings_created: r.get("listings_created"),
            status: r.get("status"),
            hardware_fingerprint: r.get("hardware_fingerprint"),
            verified_at: r.get("verified_at"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    async fn update_trust_level(
        &self,
        seller_account_id: &str,
        trust_level: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let row = sqlx::query(
            "UPDATE seller_accounts SET trust_level = $1, updated_at = now() WHERE seller_account_id = $2 RETURNING seller_account_id, owner_id, trust_level, quota_override, listings_created, status, hardware_fingerprint, verified_at, created_at, updated_at",
        )
        .bind(trust_level)
        .bind(seller_account_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(row.map(|r| SellerAccountRow {
            seller_account_id: r.get("seller_account_id"),
            owner_id: r.get("owner_id"),
            trust_level: r.get("trust_level"),
            quota_override: r.get("quota_override"),
            listings_created: r.get("listings_created"),
            status: r.get("status"),
            hardware_fingerprint: r.get("hardware_fingerprint"),
            verified_at: r.get("verified_at"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    async fn update_quota_override(
        &self,
        seller_account_id: &str,
        quota_override: Option<i32>,
    ) -> Result<Option<SellerAccountRow>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let row = sqlx::query(
            "UPDATE seller_accounts SET quota_override = $1, updated_at = now() WHERE seller_account_id = $2 RETURNING seller_account_id, owner_id, trust_level, quota_override, listings_created, status, hardware_fingerprint, verified_at, created_at, updated_at",
        )
        .bind(quota_override)
        .bind(seller_account_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(row.map(|r| SellerAccountRow {
            seller_account_id: r.get("seller_account_id"),
            owner_id: r.get("owner_id"),
            trust_level: r.get("trust_level"),
            quota_override: r.get("quota_override"),
            listings_created: r.get("listings_created"),
            status: r.get("status"),
            hardware_fingerprint: r.get("hardware_fingerprint"),
            verified_at: r.get("verified_at"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    async fn increment_listings_created(
        &self,
        seller_account_id: &str,
    ) -> Result<Option<SellerAccountRow>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        let row = sqlx::query(
            "UPDATE seller_accounts SET listings_created = listings_created + 1, updated_at = now() WHERE seller_account_id = $1 RETURNING seller_account_id, owner_id, trust_level, quota_override, listings_created, status, hardware_fingerprint, verified_at, created_at, updated_at",
        )
        .bind(seller_account_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(row.map(|r| SellerAccountRow {
            seller_account_id: r.get("seller_account_id"),
            owner_id: r.get("owner_id"),
            trust_level: r.get("trust_level"),
            quota_override: r.get("quota_override"),
            listings_created: r.get("listings_created"),
            status: r.get("status"),
            hardware_fingerprint: r.get("hardware_fingerprint"),
            verified_at: r.get("verified_at"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }
}
