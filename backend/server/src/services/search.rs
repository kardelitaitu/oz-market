use crate::auth::AuthzError;
use crate::repositories::{ListingRepository, RepositoryError};
use marketplace_api_contract::{
    ListingPayload, ListingSummary, SearchRequest, SearchResponse, SearchSort,
};
use marketplace_auth_core::Claims;
use std::cmp::Ordering;
use std::sync::Arc;

pub struct SearchService<R> {
    repository: Arc<R>,
}

impl<R> SearchService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> SearchService<R>
where
    R: ListingRepository + Send + Sync,
{
    pub async fn search_listings(
        &self,
        claims: &Claims,
        request: &SearchRequest,
    ) -> Result<SearchResponse, SearchError> {
        crate::services::authz::authorize_search_listings(claims)?;
        self.repository
            .as_ref()
            .search_listings(request)
            .await
            .map_err(SearchError::from)
    }

    pub async fn get_listing(
        &self,
        claims: &Claims,
        listing_id: &str,
    ) -> Result<Option<ListingSummary>, SearchError> {
        crate::services::authz::authorize_get_listing(claims)?;
        self.repository
            .as_ref()
            .get_listing(listing_id)
            .await
            .map_err(SearchError::from)
    }
}

#[derive(Debug)]
pub enum SearchError {
    Authz(AuthzError),
    Storage(RepositoryError),
}

impl From<AuthzError> for SearchError {
    fn from(value: AuthzError) -> Self {
        Self::Authz(value)
    }
}

impl From<RepositoryError> for SearchError {
    fn from(value: RepositoryError) -> Self {
        Self::Storage(value)
    }
}

pub fn normalize_search_terms(input: &str) -> Vec<String> {
    input
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter_map(|part| {
            let token = part.trim().to_ascii_lowercase();
            if token.is_empty() {
                None
            } else {
                Some(token)
            }
        })
        .collect()
}

pub fn listing_index_text(listing: &ListingPayload) -> String {
    let mut parts = vec![
        listing.title.clone(),
        format!("{:?}", listing.category),
        format!("{:?}", listing.condition),
        listing.location.country_name.clone(),
        listing.location.city.clone(),
        listing.description.clone(),
    ];

    if let Some(attributes) = &listing.attributes {
        if let Some(object) = attributes.as_object() {
            for (key, value) in object {
                parts.push(key.clone());
                parts.push(value.to_string());
            }
        }
    }

    parts.join(" ")
}

pub fn score_listing(listing: &ListingSummary, query_terms: &[String]) -> i64 {
    if query_terms.is_empty() {
        return 0;
    }

    let haystack = listing_index_text(&listing.listing).to_ascii_lowercase();
    let mut score = 0i64;

    for term in query_terms {
        if haystack.contains(term) {
            score += 10;
        }
        if listing.listing.title.to_ascii_lowercase().contains(term) {
            score += 20;
        }
        if listing
            .listing
            .location
            .city
            .to_ascii_lowercase()
            .contains(term)
        {
            score += 8;
        }
        if listing
            .listing
            .location
            .country_name
            .to_ascii_lowercase()
            .contains(term)
        {
            score += 8;
        }
    }

    score
}

pub fn compare_search_items(
    a: &ListingSummary,
    b: &ListingSummary,
    query_terms: &[String],
    sort_by: SearchSort,
) -> Ordering {
    match sort_by {
        SearchSort::Relevance => {
            let score_a = score_listing(a, query_terms);
            let score_b = score_listing(b, query_terms);
            score_b
                .cmp(&score_a)
                .then_with(|| a.listing.title.cmp(&b.listing.title))
                .then_with(|| a.listing_id.cmp(&b.listing_id))
        }
        SearchSort::Newest => b
            .version
            .cmp(&a.version)
            .then_with(|| a.listing_id.cmp(&b.listing_id)),
        SearchSort::PriceAsc => a
            .listing
            .price
            .amount
            .partial_cmp(&b.listing.price.amount)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.listing_id.cmp(&b.listing_id)),
        SearchSort::PriceDesc => b
            .listing
            .price
            .amount
            .partial_cmp(&a.listing.price.amount)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.listing_id.cmp(&b.listing_id)),
        // Phase B: Rating sort
        SearchSort::RatingHighest => {
            // Sort by seller_rating descending (highest first)
            let rating_a = a.seller_rating.unwrap_or(0.0);
            let rating_b = b.seller_rating.unwrap_or(0.0);
            rating_b
                .partial_cmp(&rating_a)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.listing_id.cmp(&b.listing_id))
        }
        SearchSort::RatingLowest => {
            // Sort by seller_rating ascending (lowest first)
            let rating_a = a.seller_rating.unwrap_or(0.0);
            let rating_b = b.seller_rating.unwrap_or(0.0);
            rating_a
                .partial_cmp(&rating_b)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.listing_id.cmp(&b.listing_id))
        }
        // NEW: Phase 2 - Price per sqm sorts
        SearchSort::PricePerSqmAsc => {
            // Calculate price per sqm for both items
            let price_per_sqm_a =
                a.listing.price.amount / a.listing.area_sqm.unwrap_or(0.0).max(0.01); // Avoid division by zero
            let price_per_sqm_b =
                b.listing.price.amount / b.listing.area_sqm.unwrap_or(0.0).max(0.01);
            price_per_sqm_a
                .partial_cmp(&price_per_sqm_b)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.listing_id.cmp(&b.listing_id))
        }
        SearchSort::PricePerSqmDesc => {
            // Calculate price per sqm for both items
            let price_per_sqm_a =
                a.listing.price.amount / a.listing.area_sqm.unwrap_or(0.0).max(0.01); // Avoid division by zero
            let price_per_sqm_b =
                b.listing.price.amount / b.listing.area_sqm.unwrap_or(0.0).max(0.01);
            price_per_sqm_b
                .partial_cmp(&price_per_sqm_a)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.listing_id.cmp(&b.listing_id))
        }
    }
}
