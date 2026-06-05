use oz_market_api_contract::*;
use serde_json::Value;

#[test]
fn integration_search_request_serialisation() {
    let req = SearchRequest {
        query: Some("phone".into()),
        category: Some(Category::Phone),
        price: Some(SearchPriceFilter {
            currency: Some("EUR".into()),
            min_amount: Some(100.0),
            max_amount: Some(500.0),
        }),
        location: Some(SearchLocationFilter {
            country_code: Some("DE".into()),
            city: Some("Berlin".into()),
        }),
        status: Some(ListingStatus::Active),
        sort_by: SearchSort::PriceDesc,
        limit: Some(5),
        // New service filter
        service_type: Some(ServiceType::Local),
        // New property filter (but not used here)
        property_transaction_type: None,
        // Geolocation fields left None
        ..Default::default()
    };

    // Serialize and ensure new fields are present in JSON output
    let json: Value = serde_json::to_value(&req).expect("serialize");
    assert_eq!(json.get("service_type").unwrap().as_str(), Some("local"));
    // Ensure optional fields not set are omitted
    assert!(json.get("property_transaction_type").is_none());
    assert!(json.get("is_near_me").is_none());
}

#[test]
fn integration_search_request_default_behaviour() {
    let default_req = SearchRequest::default();
    // Ensure default sort
    assert_eq!(default_req.sort_by, SearchSort::Relevance);
    // Optional fields should be None / empty
    assert!(default_req.service_type.is_none());
    assert!(default_req.property_transaction_type.is_none());
}
