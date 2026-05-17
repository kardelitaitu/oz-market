use marketplace_api_contract::{
    Category, Condition, CreateListingRequest, ListingStatus, ListingType, OpenNegotiationRequest,
    RequestContactRevealRequest, SearchRequest, SearchSort,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::path::PathBuf;

fn read_json_file(relative_path: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {}", path.display(), error));
    serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("failed to parse {} as JSON: {}", path.display(), error))
}

fn round_trip<T>(json: &str) -> T
where
    T: DeserializeOwned + Serialize + PartialEq + Debug,
{
    let parsed: T = serde_json::from_str(json).expect("failed to parse fixture JSON");
    let encoded = serde_json::to_string(&parsed).expect("failed to serialize fixture JSON");
    let reparsed: T = serde_json::from_str(&encoded).expect("failed to reparse fixture JSON");
    assert_eq!(parsed, reparsed);
    parsed
}

#[test]
fn android_and_ios_manifests_point_to_the_same_openapi_contract() {
    let android = read_json_file("../../../mobile/app-android/contract-manifest.json");
    let ios = read_json_file("../../../mobile/app-ios/contract-manifest.json");

    assert_eq!(android["contract_source"], ios["contract_source"]);
    assert_eq!(android["contract_source"], "../../docs/specs/openapi.yaml");
    assert_eq!(android["platform"], "android");
    assert_eq!(ios["platform"], "ios");
}

#[test]
fn mobile_payloads_round_trip_through_shared_contract_types() {
    let create_request_json = r#"{
        "idempotency_key": "idem-create-mobile-1",
        "listing": {
            "schema_version": "1.0",
            "owner_id": "seller-123",
            "listing_type": "product",
            "category": "laptop",
            "title": "Lenovo ThinkPad T480",
            "condition": "used",
            "price": {
                "currency": "USD",
                "amount": 450.0
            },
            "location": {
                "country_code": "US",
                "country_name": "United States",
                "city": "Austin"
            },
            "picture_urls": [
                "https://example.com/item.jpg"
            ],
            "description": "Good battery health"
        }
    }"#;
    let create_request: CreateListingRequest = round_trip(create_request_json);
    assert_eq!(create_request.listing.listing_type, ListingType::Product);
    assert_eq!(create_request.listing.category, Some(Category::Laptop));
    assert_eq!(create_request.listing.condition, Some(Condition::Used));

    let search_request_json = r#"{
        "query": "thinkpad",
        "category": "laptop",
        "condition": "used",
        "status": "active",
        "listing_type": "product",
        "limit": 20,
        "sort_by": "relevance"
    }"#;
    let search_request: SearchRequest = round_trip(search_request_json);
    assert_eq!(search_request.status, Some(ListingStatus::Active));
    assert_eq!(search_request.limit, Some(20));
    assert_eq!(search_request.sort_by, SearchSort::Relevance);

    let open_negotiation_json = r#"{
        "listing_id": "lst_123",
        "buyer_agent_id": "buyer-123",
        "offer_currency": "USD",
        "offer_amount": 440.0,
        "idempotency_key": "idem-open-mobile-1"
    }"#;
    let open_negotiation: OpenNegotiationRequest = round_trip(open_negotiation_json);
    assert_eq!(open_negotiation.listing_id, "lst_123");
    assert_eq!(open_negotiation.buyer_agent_id, "buyer-123");

    let reveal_request_json = r#"{
        "idempotency_key": "idem-reveal-mobile-1"
    }"#;
    let reveal_request: RequestContactRevealRequest = round_trip(reveal_request_json);
    assert_eq!(reveal_request.idempotency_key, "idem-reveal-mobile-1");
}
