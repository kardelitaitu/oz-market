use crate::models::db::ReviewRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};
use async_trait::async_trait;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::RwLock;

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait ReviewRepository: Send + Sync {
    async fn create_review(
        &self,
        review_id: &str,
        listing_id: &str,
        seller_account_id: &str,
        reviewer_id: &str,
        rating: i32,
        title: &str,
        body: Option<&str>,
    ) -> Result<ReviewRow, RepositoryError>;

    async fn get_reviews_for_listing(
        &self,
        listing_id: &str,
    ) -> Result<Vec<ReviewRow>, RepositoryError>;

    async fn get_reviews_for_seller(
        &self,
        seller_account_id: &str,
    ) -> Result<Vec<ReviewRow>, RepositoryError>;

    async fn update_review_status(
        &self,
        review_id: &str,
        status: &str,
    ) -> Result<Option<ReviewRow>, RepositoryError>;

    async fn get_by_id(&self, review_id: &str) -> Result<Option<ReviewRow>, RepositoryError>;
}

// InMemory Implementation
#[derive(Default)]
pub struct InMemoryReviewRepository {
    reviews: RwLock<HashMap<String, ReviewRow>>,
}

impl InMemoryReviewRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ReviewRepository for InMemoryReviewRepository {
    async fn create_review(
        &self,
        review_id: &str,
        listing_id: &str,
        seller_account_id: &str,
        reviewer_id: &str,
        rating: i32,
        title: &str,
        body: Option<&str>,
    ) -> Result<ReviewRow, RepositoryError> {
        let mut guard = self.reviews.write().expect("review write lock");

        let now = chrono::Utc::now().to_rfc3339();
        let review = ReviewRow {
            review_id: review_id.to_string(),
            listing_id: listing_id.to_string(),
            seller_account_id: seller_account_id.to_string(),
            reviewer_id: reviewer_id.to_string(),
            rating,
            title: title.to_string(),
            body: body.map(|s| s.to_string()),
            status: "pending".to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        guard.insert(review_id.to_string(), review.clone());
        Ok(review)
    }

    async fn get_reviews_for_listing(
        &self,
        listing_id: &str,
    ) -> Result<Vec<ReviewRow>, RepositoryError> {
        let guard = self.reviews.read().expect("review read lock");
        let reviews: Vec<ReviewRow> = guard
            .values()
            .filter(|r| r.listing_id == listing_id)
            .cloned()
            .collect();
        Ok(reviews)
    }

    async fn get_reviews_for_seller(
        &self,
        seller_account_id: &str,
    ) -> Result<Vec<ReviewRow>, RepositoryError> {
        let guard = self.reviews.read().expect("review read lock");
        let reviews: Vec<ReviewRow> = guard
            .values()
            .filter(|r| r.seller_account_id == seller_account_id)
            .cloned()
            .collect();
        Ok(reviews)
    }

    async fn update_review_status(
        &self,
        review_id: &str,
        status: &str,
    ) -> Result<Option<ReviewRow>, RepositoryError> {
        let mut guard = self.reviews.write().expect("review write lock");
        if let Some(review) = guard.get_mut(review_id) {
            review.status = status.to_string();
            review.updated_at = chrono::Utc::now().to_rfc3339();
            Ok(Some(review.clone()))
        } else {
            Ok(None)
        }
    }

    async fn get_by_id(&self, review_id: &str) -> Result<Option<ReviewRow>, RepositoryError> {
        let guard = self.reviews.read().expect("review read lock");
        Ok(guard.get(review_id).cloned())
    }
}

// Postgres Implementation
pub struct PostgresReviewRepository {
    pool: sqlx::postgres::PgPool,
}

impl PostgresReviewRepository {
    pub fn new(pool: sqlx::postgres::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReviewRepository for PostgresReviewRepository {
    async fn create_review(
        &self,
        review_id: &str,
        listing_id: &str,
        seller_account_id: &str,
        reviewer_id: &str,
        rating: i32,
        title: &str,
        body: Option<&str>,
    ) -> Result<ReviewRow, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let row = sqlx::query(
            "INSERT INTO reviews (review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', now(), now())
             RETURNING review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status, created_at, updated_at"
        )
        .bind(review_id)
        .bind(listing_id)
        .bind(seller_account_id)
        .bind(reviewer_id)
        .bind(rating)
        .bind(title)
        .bind(body)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        Ok(ReviewRow {
            review_id: row.get("review_id"),
            listing_id: row.get("listing_id"),
            seller_account_id: row.get("seller_account_id"),
            reviewer_id: row.get("reviewer_id"),
            rating: row.get("rating"),
            title: row.get("title"),
            body: row.get("body"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn get_reviews_for_listing(
        &self,
        listing_id: &str,
    ) -> Result<Vec<ReviewRow>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let rows = sqlx::query(
            "SELECT review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status, created_at, updated_at
             FROM reviews WHERE listing_id = $1 ORDER BY created_at DESC"
        )
        .bind(listing_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let reviews = rows
            .into_iter()
            .map(|row| ReviewRow {
                review_id: row.get("review_id"),
                listing_id: row.get("listing_id"),
                seller_account_id: row.get("seller_account_id"),
                reviewer_id: row.get("reviewer_id"),
                rating: row.get("rating"),
                title: row.get("title"),
                body: row.get("body"),
                status: row.get("status"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(reviews)
    }

    async fn get_reviews_for_seller(
        &self,
        seller_account_id: &str,
    ) -> Result<Vec<ReviewRow>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let rows = sqlx::query(
            "SELECT review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status, created_at, updated_at
             FROM reviews WHERE seller_account_id = $1 ORDER BY created_at DESC"
        )
        .bind(seller_account_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let reviews = rows
            .into_iter()
            .map(|row| ReviewRow {
                review_id: row.get("review_id"),
                listing_id: row.get("listing_id"),
                seller_account_id: row.get("seller_account_id"),
                reviewer_id: row.get("reviewer_id"),
                rating: row.get("rating"),
                title: row.get("title"),
                body: row.get("body"),
                status: row.get("status"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(reviews)
    }

    async fn update_review_status(
        &self,
        review_id: &str,
        status: &str,
    ) -> Result<Option<ReviewRow>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let row = sqlx::query(
            "UPDATE reviews SET status = $1, updated_at = now() WHERE review_id = $2
             RETURNING review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status, created_at, updated_at"
        )
        .bind(status)
        .bind(review_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        Ok(row.map(|row| ReviewRow {
            review_id: row.get("review_id"),
            listing_id: row.get("listing_id"),
            seller_account_id: row.get("seller_account_id"),
            reviewer_id: row.get("reviewer_id"),
            rating: row.get("rating"),
            title: row.get("title"),
            body: row.get("body"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    async fn get_by_id(&self, review_id: &str) -> Result<Option<ReviewRow>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        let row = sqlx::query(
            "SELECT review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status, created_at, updated_at
             FROM reviews WHERE review_id = $1"
        )
        .bind(review_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| RepositoryError::new(RepositoryErrorKind::Storage, e.to_string()))?;

        Ok(row.map(|row| ReviewRow {
            review_id: row.get("review_id"),
            listing_id: row.get("listing_id"),
            seller_account_id: row.get("seller_account_id"),
            reviewer_id: row.get("reviewer_id"),
            rating: row.get("rating"),
            title: row.get("title"),
            body: row.get("body"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_create_review_success() {
        let repo = InMemoryReviewRepository::new();
        let result = repo
            .create_review(
                "rev_123",
                "lst_456",
                "seller_789",
                "buyer_101",
                5,
                "Great product",
                Some("Highly recommend"),
            )
            .await;
        assert!(result.is_ok());
        let review = result.unwrap();
        assert_eq!(review.review_id, "rev_123");
        assert_eq!(review.rating, 5);
        assert_eq!(review.status, "pending");
    }

    #[tokio::test]
    async fn test_in_memory_get_reviews_for_listing() {
        let repo = InMemoryReviewRepository::new();
        repo.create_review(
            "rev_123",
            "lst_456",
            "seller_789",
            "buyer_101",
            5,
            "Great",
            None,
        )
        .await
        .unwrap();
        repo.create_review(
            "rev_124",
            "lst_789",
            "seller_999",
            "buyer_102",
            4,
            "Good",
            None,
        )
        .await
        .unwrap();

        let result = repo.get_reviews_for_listing("lst_456").await;
        assert!(result.is_ok());
        let reviews = result.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].review_id, "rev_123");
    }

    #[tokio::test]
    async fn test_in_memory_get_reviews_for_seller() {
        let repo = InMemoryReviewRepository::new();
        repo.create_review(
            "rev_123",
            "lst_456",
            "seller_789",
            "buyer_101",
            5,
            "Great",
            None,
        )
        .await
        .unwrap();

        let result = repo.get_reviews_for_seller("seller_789").await;
        assert!(result.is_ok());
        let reviews = result.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].seller_account_id, "seller_789");
    }

    #[tokio::test]
    async fn test_in_memory_update_review_status() {
        let repo = InMemoryReviewRepository::new();
        repo.create_review(
            "rev_123",
            "lst_456",
            "seller_789",
            "buyer_101",
            5,
            "Great",
            None,
        )
        .await
        .unwrap();

        let result = repo.update_review_status("rev_123", "approved").await;
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().status, "approved");
    }

    #[tokio::test]
    async fn test_in_memory_update_review_status_not_found() {
        let repo = InMemoryReviewRepository::new();
        let result = repo.update_review_status("nonexistent", "approved").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_in_memory_get_by_id_found() {
        let repo = InMemoryReviewRepository::new();
        repo.create_review(
            "rev_123",
            "lst_456",
            "seller_789",
            "buyer_101",
            5,
            "Great",
            None,
        )
        .await
        .unwrap();

        let result = repo.get_by_id("rev_123").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_in_memory_get_by_id_not_found() {
        let repo = InMemoryReviewRepository::new();
        let result = repo.get_by_id("nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
