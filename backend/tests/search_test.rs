#!/usr/bin/env rustc -L '/path/to/toolchain'

use marketplace_api_contract::*;
use serde_json::json;

#[test]
fn test_search_api_contract() {
    // Test search payload round-trip with all feature flags
    let search_req = SearchRequest {
        query: Some("laptop"),
        category: Some(Category::Laptop),
        price: Some(SearchPriceFilter {
            min_amount: Some(500.0),
            max_amount: Some(1500.0),
            currency: Some("USD"),
        }),
        location: Some(SearchLocationFilter {
            country_code: Some("US"),
            city: Some("San Francisco"),
        }),
        status: Some(ListingStatus::Active),
        sort_by: SearchSort::PriceAsc,
        limit: Some(10),
        // NEW fields for service type and property filters
        service_type: Some(ServiceType::Online),
        property_transaction_type: Some(PropertyTransactionType::Rent),
        // Phase D geolocation
        near_me: Some(true),
        user_latitude: Some(37.7749),
        user_longitude: Some(-122.4194),
        radius_km: Some(10.0),
    };

    // Serialize to JSON
    let json_val = serde_json::to_value(&search_req).expect("Serialization failed");
    assert_eq!(json_val.get("query").unwrap().as_str(), Some(&"laptop"));
    assert_eq!(json_val.get("category").unwrap().as_str(), Some(&"laptop"));
    // Verify advanced filters
    assert_eq!(json_val.get("property_transaction_type").unwrap().as_str(), Some(&"rent"));
    // Check geolocation parameters
    assert_eq!(json_val.get("near_me").unwrap().as_bool(), Some(&true));
}

#[test]
fn test_search_empty_query() {
    let empty_search = SearchRequest::default();
    // Verify defaults
    assert_eq!(empty_search.sort_by, SearchSort::Relevance);
    assert!(empty_search.query.is_none());
    assert!(empty_search.category.is_none());
}