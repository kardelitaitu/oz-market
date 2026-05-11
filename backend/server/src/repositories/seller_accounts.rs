use crate::models::db::SellerAccountRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::RwLock;

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}

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

impl Default for InMemorySellerAccountRepository {
    fn default() -> Self {
        Self {
            accounts: RwLock::new(HashMap::new()),
        }
    }
}

impl InMemorySellerAccountRepository {
    pub fn new() -> Self {
        Self::default()
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
            "SELECT seller_account_id, owner_id, display_name, trust_level, seller_rating::TEXT AS seller_rating, quota_override, listings_created, status, hardware_fingerprint, verified_at::TEXT AS verified_at, created_at::TEXT AS created_at, updated_at::TEXT AS updated_at FROM seller_accounts WHERE owner_id = $1",
        )
        .bind(owner_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(row.map(|r| SellerAccountRow {
            seller_account_id: r.get("seller_account_id"),
            owner_id: r.get("owner_id"),
            display_name: r.get("display_name"),
            trust_level: r.get("trust_level"),
            seller_rating: r
                .try_get::<Option<String>, _>("seller_rating")
                .unwrap_or(None)
                .and_then(|s: String| s.parse::<f64>().ok()),
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
            "UPDATE seller_accounts SET trust_level = $1, updated_at = now() WHERE seller_account_id = $2 RETURNING seller_account_id, owner_id, display_name, trust_level, seller_rating::TEXT AS seller_rating, quota_override, listings_created, status, hardware_fingerprint, verified_at::TEXT AS verified_at, created_at::TEXT AS created_at, updated_at::TEXT AS updated_at",
        )
        .bind(trust_level)
        .bind(seller_account_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(row.map(|r| SellerAccountRow {
            seller_account_id: r.get("seller_account_id"),
            owner_id: r.get("owner_id"),
            display_name: r.get("display_name"),
            trust_level: r.get("trust_level"),
            seller_rating: r
                .try_get::<Option<String>, _>("seller_rating")
                .unwrap_or(None)
                .and_then(|s: String| s.parse::<f64>().ok()),
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
            "UPDATE seller_accounts SET quota_override = $1, updated_at = now() WHERE seller_account_id = $2 RETURNING seller_account_id, owner_id, display_name, trust_level, seller_rating::TEXT AS seller_rating, quota_override, listings_created, status, hardware_fingerprint, verified_at::TEXT AS verified_at, created_at::TEXT AS created_at, updated_at::TEXT AS updated_at",
        )
        .bind(quota_override)
        .bind(seller_account_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(row.map(|r| SellerAccountRow {
            seller_account_id: r.get("seller_account_id"),
            owner_id: r.get("owner_id"),
            display_name: r.get("display_name"),
            trust_level: r.get("trust_level"),
            seller_rating: r
                .try_get::<Option<String>, _>("seller_rating")
                .unwrap_or(None)
                .and_then(|s: String| s.parse::<f64>().ok()),
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
            "UPDATE seller_accounts SET listings_created = listings_created + 1, updated_at = now() WHERE seller_account_id = $1 RETURNING seller_account_id, owner_id, display_name, trust_level, seller_rating::TEXT AS seller_rating, quota_override, listings_created, status, hardware_fingerprint, verified_at::TEXT AS verified_at, created_at::TEXT AS created_at, updated_at::TEXT AS updated_at",
        )
        .bind(seller_account_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;
        Ok(row.map(|r| SellerAccountRow {
            seller_account_id: r.get("seller_account_id"),
            owner_id: r.get("owner_id"),
            display_name: r.get("display_name"),
            trust_level: r.get("trust_level"),
            seller_rating: r
                .try_get::<Option<String>, _>("seller_rating")
                .unwrap_or(None)
                .and_then(|s: String| s.parse::<f64>().ok()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_account(id: &str, owner: &str) -> SellerAccountRow {
        SellerAccountRow {
            seller_account_id: id.to_string(),
            owner_id: owner.to_string(),
            display_name: Some(owner.to_string()),
            trust_level: "basic".to_string(),
            seller_rating: Some(4.5),
            quota_override: None,
            listings_created: 5,
            status: "active".to_string(),
            hardware_fingerprint: None,
            verified_at: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn get_by_owner_id_found() {
        let repo = InMemorySellerAccountRepository::new();
        repo.add_account(sample_account("sa_1", "owner-1"));
        let result = repo.get_by_owner_id("owner-1").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().seller_account_id, "sa_1");
    }

    #[tokio::test]
    async fn get_by_owner_id_not_found_returns_none() {
        let repo = InMemorySellerAccountRepository::new();
        let result = repo.get_by_owner_id("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn update_trust_level_updates() {
        let repo = InMemorySellerAccountRepository::new();
        repo.add_account(sample_account("sa_1", "owner-1"));
        let updated = repo.update_trust_level("sa_1", "premium").await.unwrap();
        assert_eq!(updated.unwrap().trust_level, "premium");
    }

    #[tokio::test]
    async fn update_trust_level_not_found_returns_none() {
        let repo = InMemorySellerAccountRepository::new();
        let result = repo
            .update_trust_level("sa_nonexistent", "premium")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn update_quota_override_sets_quota() {
        let repo = InMemorySellerAccountRepository::new();
        repo.add_account(sample_account("sa_1", "owner-1"));
        let updated = repo.update_quota_override("sa_1", Some(100)).await.unwrap();
        assert_eq!(updated.unwrap().quota_override, Some(100));
    }

    #[tokio::test]
    async fn update_quota_override_clears_quota() {
        let repo = InMemorySellerAccountRepository::new();
        let mut account = sample_account("sa_1", "owner-1");
        account.quota_override = Some(50);
        repo.add_account(account);
        let updated = repo.update_quota_override("sa_1", None).await.unwrap();
        assert_eq!(updated.unwrap().quota_override, None);
    }

    #[tokio::test]
    async fn update_quota_override_not_found_returns_none() {
        let repo = InMemorySellerAccountRepository::new();
        let result = repo
            .update_quota_override("sa_nonexistent", Some(100))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn increment_listings_created_increments() {
        let repo = InMemorySellerAccountRepository::new();
        repo.add_account(sample_account("sa_1", "owner-1"));
        let updated = repo.increment_listings_created("sa_1").await.unwrap();
        assert_eq!(updated.unwrap().listings_created, 6);
    }

    #[tokio::test]
    async fn increment_listings_created_twice() {
        let repo = InMemorySellerAccountRepository::new();
        repo.add_account(sample_account("sa_1", "owner-1"));
        repo.increment_listings_created("sa_1").await.unwrap();
        let updated = repo.increment_listings_created("sa_1").await.unwrap();
        assert_eq!(updated.unwrap().listings_created, 7);
    }

    #[tokio::test]
    async fn increment_listings_created_not_found_returns_none() {
        let repo = InMemorySellerAccountRepository::new();
        let result = repo
            .increment_listings_created("sa_nonexistent")
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
