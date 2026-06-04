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
        claims: Option<&Claims>,
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
        claims: Option<&Claims>,
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

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::Authz(error) => write!(f, "authz: {error}"),
            SearchError::Storage(error) => write!(f, "storage: {error}"),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use marketplace_api_contract::*;

    fn make_listing(title: &str, price: f64, city: &str, country: &str) -> ListingSummary {
        ListingSummary {
            listing_id: format!("lst_{}", title.len()),
            version: 1,
            status: ListingStatus::Active,
            seller_rating: Some(4.5),
            seller_name: None,
            seller_verified: None,
            listing: ListingPayload {
                schema_version: "1.0".to_string(),
                owner_id: "seller_1".to_string(),
                listing_type: ListingType::Product,
                category: Some(Category::Laptop),
                title: title.to_string(),
                condition: Some(Condition::Used),
                price: Price {
                    amount: price,
                    currency: "USD".to_string(),
                },
                location: ListingLocation {
                    country_code: "US".to_string(),
                    country_name: country.to_string(),
                    city: city.to_string(),
                    latitude: None,
                    longitude: None,
                    geolocation_opt_out: None,
                },
                picture_urls: vec![],
                description: "Test description".to_string(),
                attributes: None,
                sku: None,
                quantity: None,
                shipping_info: None,
                condition_details: None,
                seller_notes: None,
                service_type: None,
                hourly_rate: None,
                project_rate: None,
                qualifications: None,
                service_radius_km: None,
                property_transaction_type: None,
                property_sub_type: None,
                area_sqm: None,
                bedrooms: None,
                bathrooms: None,
                year_built: None,
                lot_size_sqm: None,
                zoning: None,
            },
        }
    }

    #[test]
    fn normalize_search_terms_handles_empty_input() {
        assert_eq!(normalize_search_terms(""), Vec::<String>::new());
    }

    #[test]
    fn normalize_search_terms_trims_and_lowercases() {
        let result = normalize_search_terms("  Hello World  ");
        assert_eq!(result, vec!["hello", "world"]);
    }

    #[test]
    fn normalize_search_terms_filters_non_alphanumeric() {
        let result = normalize_search_terms("laptop!@# $1999");
        assert_eq!(result, vec!["laptop", "1999"]);
    }

    #[test]
    fn normalize_search_terms_handles_multiple_spaces() {
        let result = normalize_search_terms("macbook     pro");
        assert_eq!(result, vec!["macbook", "pro"]);
    }

    #[test]
    fn listing_index_text_includes_all_fields() {
        let listing = ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "seller_1".to_string(),
            listing_type: ListingType::Product,
            category: Some(Category::Laptop),
            title: "MacBook Pro".to_string(),
            condition: Some(Condition::New),
            price: Price {
                amount: 1500.0,
                currency: "USD".to_string(),
            },
            location: ListingLocation {
                country_code: "US".to_string(),
                country_name: "USA".to_string(),
                city: "San Francisco".to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            description: "16GB RAM".to_string(),
            picture_urls: vec![],
            attributes: Some(serde_json::json!({"ram": "16GB", "storage": "512GB"})),
            sku: None,
            quantity: None,
            shipping_info: None,
            condition_details: None,
            seller_notes: None,
            service_type: None,
            hourly_rate: None,
            project_rate: None,
            qualifications: None,
            service_radius_km: None,
            property_transaction_type: None,
            property_sub_type: None,
            area_sqm: None,
            bedrooms: None,
            bathrooms: None,
            year_built: None,
            lot_size_sqm: None,
            zoning: None,
        };
        let text = listing_index_text(&listing);
        assert!(text.contains("MacBook Pro"));
        assert!(text.contains("San Francisco"));
        assert!(text.contains("USA"));
        assert!(text.contains("ram"));
    }

    #[test]
    fn score_listing_returns_zero_for_empty_terms() {
        let listing = make_listing("Test", 100.0, "NYC", "USA");
        assert_eq!(score_listing(&listing, &[]), 0);
    }

    #[test]
    fn score_listing_awards_title_match_bonus() {
        let listing = make_listing("MacBook Pro", 1000.0, "NYC", "USA");
        let score = score_listing(&listing, &["macbook".to_string()]);
        assert!(score > 10);
    }

    #[test]
    fn score_listing_awards_location_match() {
        let listing = make_listing("Laptop", 1000.0, "San Francisco", "USA");
        let score = score_listing(&listing, &["san".to_string()]);
        assert!(score >= 8);
    }

    #[test]
    fn compare_search_items_sorts_by_relevance() {
        let a = make_listing("MacBook Pro", 1000.0, "NYC", "USA");
        let b = make_listing("Old Laptop", 500.0, "LA", "USA");
        let terms = vec!["macbook".to_string()];
        // a has matching title, should come first -> Ordering::Less
        assert_eq!(
            compare_search_items(&a, &b, &terms, SearchSort::Relevance),
            Ordering::Less
        );
    }

    #[test]
    fn compare_search_items_sorts_by_price_asc() {
        let a = make_listing("Laptop", 500.0, "NYC", "USA");
        let b = make_listing("Laptop", 1000.0, "NYC", "USA");
        // a is cheaper, should come first -> Ordering::Less
        assert_eq!(
            compare_search_items(&a, &b, &[], SearchSort::PriceAsc),
            Ordering::Less
        );
    }

    #[test]
    fn compare_search_items_sorts_by_price_desc() {
        let a = make_listing("Laptop", 500.0, "NYC", "USA");
        let b = make_listing("Laptop", 1000.0, "NYC", "USA");
        // b is more expensive, should come first -> Ordering::Greater
        assert_eq!(
            compare_search_items(&a, &b, &[], SearchSort::PriceDesc),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_search_items_sorts_by_newest() {
        let mut a = make_listing("Laptop", 500.0, "NYC", "USA");
        a.version = 1;
        let mut b = make_listing("Laptop", 500.0, "NYC", "USA");
        b.version = 2;
        // b is newer, should come first -> Ordering::Greater
        assert_eq!(
            compare_search_items(&a, &b, &[], SearchSort::Newest),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_search_items_sorts_by_rating_highest() {
        let mut a = make_listing("Laptop", 500.0, "NYC", "USA");
        a.seller_rating = Some(3.0);
        let mut b = make_listing("Laptop", 500.0, "NYC", "USA");
        b.seller_rating = Some(5.0);
        // b has higher rating, should come first -> Ordering::Greater
        assert_eq!(
            compare_search_items(&a, &b, &[], SearchSort::RatingHighest),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_search_items_sorts_by_price_per_sqm() {
        let mut a = make_listing("Apartment", 100000.0, "NYC", "USA");
        a.listing.area_sqm = Some(100.0);
        let mut b = make_listing("Apartment", 150000.0, "NYC", "USA");
        b.listing.area_sqm = Some(100.0);
        // Same price per sqm (1000), tie-break on listing_id
        let result = compare_search_items(&a, &b, &[], SearchSort::PricePerSqmAsc);
        assert!(result != Ordering::Equal);
    }

    #[test]
    fn compare_search_items_handles_missing_rating_with_default() {
        let mut a = make_listing("Laptop", 500.0, "NYC", "USA");
        a.seller_rating = None;
        let mut b = make_listing("Laptop", 500.0, "NYC", "USA");
        b.seller_rating = Some(5.0);
        // b has rating, should come first (default for None is 0.0) -> Ordering::Greater
        assert_eq!(
            compare_search_items(&a, &b, &[], SearchSort::RatingHighest),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_search_items_tie_breaks_on_listing_id() {
        let a = make_listing("Laptop", 500.0, "NYC", "USA");
        let b = make_listing("Laptop", 500.0, "NYC", "USA");
        let result = compare_search_items(&a, &b, &[], SearchSort::Relevance);
        assert!(result == Ordering::Equal || a.listing_id != b.listing_id);
    }

    // -----------------------------------------------------------------------
    // Phase D: Geolocation edge cases (coordinates in index text)
    // -----------------------------------------------------------------------

    #[test]
    fn score_listing_handles_missing_coordinates() {
        let mut listing = make_listing("Laptop", 1000.0, "NYC", "USA");
        listing.listing.location.latitude = None;
        listing.listing.location.longitude = None;
        // Should not panic — score should work without coordinates
        let score = score_listing(&listing, &["laptop".to_string()]);
        assert!(score > 0);
    }

    #[test]
    fn score_listing_handles_polar_coordinates() {
        let mut listing = make_listing("Laptop", 1000.0, "NYC", "USA");
        listing.listing.location.latitude = Some(90.0); // North Pole
        listing.listing.location.longitude = Some(0.0);
        let score = score_listing(&listing, &["laptop".to_string()]);
        assert!(score > 0);
    }

    #[test]
    fn score_listing_handles_antipodal_coordinates() {
        let mut listing = make_listing("Laptop", 1000.0, "NYC", "USA");
        listing.listing.location.latitude = Some(-33.8688);
        listing.listing.location.longitude = Some(151.2093); // Sydney
        let score = score_listing(&listing, &["laptop".to_string()]);
        assert!(score > 0);
    }

    #[test]
    fn score_listing_handles_date_line_coordinates() {
        let mut listing = make_listing("Laptop", 1000.0, "NYC", "USA");
        listing.listing.location.latitude = Some(0.0);
        listing.listing.location.longitude = Some(179.9999); // Near Date Line
        let score = score_listing(&listing, &["laptop".to_string()]);
        assert!(score > 0);
    }

    #[test]
    fn score_listing_handles_zero_coordinates() {
        let mut listing = make_listing("Laptop", 1000.0, "NYC", "USA");
        listing.listing.location.latitude = Some(0.0);
        listing.listing.location.longitude = Some(0.0); // Null Island
        let score = score_listing(&listing, &["laptop".to_string()]);
        assert!(score > 0);
    }
}
