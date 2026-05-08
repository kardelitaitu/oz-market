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
