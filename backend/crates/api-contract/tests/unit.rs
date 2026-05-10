#![allow(dead_code)]

use marketplace_api_contract::*;
use serde_json::Value;

#[test]
fn test_listing_payload_roundtrip() {
    // Build minimal payload with required fields only
    let payload = ListingPayload {
        schema_version: "1.0.0".to_string(),
        owner_id: "user_123".to_string(),
        listing_type: ListingType::Product,
        category: Some(Category::Laptop),
        title: "MacBook Pro".to_string(),
        condition: Some(Condition::Used),
        price: Price {
            currency: "USD".to_string(),
            amount: 1299.99,
        },
        location: ListingLocation {
            country_code: "US".to_string(),
            country_name: "United States".to_string(),
            city: "San Francisco".to_string(),
            latitude: None,
            longitude: None,
            geolocation_opt_out: None,
        },
        picture_urls: vec![],
        description: "A used MacBook".to_string(),
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
    };

    // Serialize to JSON
    let json_val: Value = serde_json::to_value(&payload).expect("serialise failed");
    // Ensure required keys exist
    assert!(json_val.get("schema_version").is_some());
    assert!(json_val.get("owner_id").is_some());

    // Deserialise back
    let back: ListingPayload =
        serde_json::from_value(json_val.clone()).expect("deserialise failed");
    assert_eq!(payload, back);
}

#[test]
fn test_search_request_defaults() {
    let sr = SearchRequest::default();
    // default sort should be Relevance
    assert_eq!(sr.sort_by, SearchSort::Relevance);
    // all optional fields None
    assert!(sr.category.is_none());
    assert!(sr.price.is_none());
    assert!(sr.location.as_ref().and_then(|l| l.city.as_ref()).is_none());
}

#[test]
fn test_optional_fields_skip_serialisation() {
    let payload = ListingPayload {
        schema_version: "1.0.0".to_string(),
        owner_id: "id".to_string(),
        listing_type: ListingType::Service,
        category: None,
        title: "Service Title".to_string(),
        condition: None,
        price: Price {
            currency: "EUR".to_string(),
            amount: 100.0,
        },
        location: ListingLocation {
            country_code: "DE".to_string(),
            country_name: "Germany".to_string(),
            city: "Berlin".to_string(),
            latitude: None,
            longitude: None,
            geolocation_opt_out: None,
        },
        picture_urls: vec![],
        description: "My service".to_string(),
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
    };
    let json_val = serde_json::to_value(&payload).expect("serialise failed");
    // Ensure optional fields are absent
    assert!(json_val.get("category").is_none());
    assert!(json_val.get("condition").is_none());
    assert!(json_val.get("attributes").is_none());
}
