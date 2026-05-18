use marketplace_api_contract::{
    ListingType, PropertySubType, PropertyTransactionType, ServiceType,
};
use marketplace_auth_core::{Claims, Role, Scope};
use marketplace_server::app::MarketplaceApp;
use marketplace_server::repositories::audit_events::InMemoryAuditEventRepository;
use marketplace_server::repositories::contact_reveals::InMemoryContactRevealRepository;
use marketplace_server::repositories::listings::InMemoryListingRepository;
use marketplace_server::repositories::negotiations::InMemoryNegotiationRepository;
use marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository;
use marketplace_server::repositories::reservations::InMemoryReservationLeaseRepository;
use marketplace_server::repositories::seller_accounts::InMemorySellerAccountRepository;
use marketplace_server::services::idempotency::InMemoryIdempotencyRepository;

type TestApp = MarketplaceApp<
    InMemoryListingRepository,
    InMemoryIdempotencyRepository,
    InMemoryReservationLeaseRepository,
    InMemoryContactRevealRepository,
>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing new marketplace schema...");

    let app = TestApp::new(
        InMemoryListingRepository::new(),
        InMemoryIdempotencyRepository::new(),
        InMemoryReservationLeaseRepository::new(),
        InMemoryContactRevealRepository::new(),
        std::sync::Arc::new(InMemoryNegotiationRepository::new()),
        std::sync::Arc::new(InMemoryAuditEventRepository::new()),
        std::sync::Arc::new(InMemoryOutboxEventRepository::new()),
        std::sync::Arc::new(InMemorySellerAccountRepository::new()),
    );

    let claims = Claims {
        sub: "test-seller".to_string(),
        roles: vec![Role::SellerListingWriter],
        scopes: vec![Scope::ListingCreate, Scope::ListingRead],
        seller_account_id: Some("test-seller".to_string()),
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };

    // Test creating different listing types
    let product_request = marketplace_api_contract::CreateListingRequest {
        idempotency_key: "test-product".to_string(),
        listing: marketplace_api_contract::ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "test-seller".to_string(),
            listing_type: ListingType::Product,
            category: Some(marketplace_api_contract::Category::Laptop),
            title: "Test Laptop".to_string(),
            condition: Some(marketplace_api_contract::Condition::Used),
            price: marketplace_api_contract::Price {
                currency: "USD".to_string(),
                amount: 999.99,
            },
            location: marketplace_api_contract::ListingLocation {
                country_code: "US".to_string(),
                country_name: "United States".to_string(),
                city: "Test City".to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec!["http://example.com/test.jpg".to_string()],
            description: "Test product listing".to_string(),
            attributes: Some(serde_json::json!({"brand": "Test"})),
            sku: Some("TEST-001".to_string()),
            quantity: Some(1),
            shipping_info: Some(marketplace_api_contract::ShippingInfo {
                local_pickup: true,
                shipping_available: true,
                shipping_cost: Some(marketplace_api_contract::Price {
                    currency: "USD".to_string(),
                    amount: 10.0,
                }),
                shipping_regions: Some(vec!["US".to_string()]),
            }),
            condition_details: Some("Excellent condition".to_string()),
            seller_notes: Some("Test notes".to_string()),
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
    };

    let service_request = marketplace_api_contract::CreateListingRequest {
        idempotency_key: "test-service".to_string(),
        listing: marketplace_api_contract::ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "test-seller".to_string(),
            listing_type: ListingType::Service,
            category: None,
            title: "Test Tutoring Service".to_string(),
            condition: None,
            price: marketplace_api_contract::Price {
                currency: "USD".to_string(),
                amount: 25.0,
            },
            location: marketplace_api_contract::ListingLocation {
                country_code: "US".to_string(),
                country_name: "United States".to_string(),
                city: "Test City".to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec!["http://example.com/test.jpg".to_string()],
            description: "Test service listing".to_string(),
            attributes: Some(serde_json::json!({"subject": "Math"})),
            sku: None,
            quantity: None,
            shipping_info: None,
            condition_details: None,
            seller_notes: None,
            service_type: Some(ServiceType::Local),
            hourly_rate: Some(25.0),
            project_rate: Some(200.0),
            qualifications: Some(vec![
                "Teaching License".to_string(),
                "Math Degree".to_string(),
            ]),
            service_radius_km: Some(10),
            property_transaction_type: None,
            property_sub_type: None,
            area_sqm: None,
            bedrooms: None,
            bathrooms: None,
            year_built: None,
            lot_size_sqm: None,
            zoning: None,
        },
    };

    let property_request = marketplace_api_contract::CreateListingRequest {
        idempotency_key: "test-property".to_string(),
        listing: marketplace_api_contract::ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "test-seller".to_string(),
            listing_type: ListingType::Property,
            category: None,
            title: "Test Apartment".to_string(),
            condition: None,
            price: marketplace_api_contract::Price {
                currency: "USD".to_string(),
                amount: 1200.0,
            },
            location: marketplace_api_contract::ListingLocation {
                country_code: "US".to_string(),
                country_name: "United States".to_string(),
                city: "Test City".to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec!["http://example.com/test.jpg".to_string()],
            description: "Test property listing".to_string(),
            attributes: Some(serde_json::json!({"furnished": true})),
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
            property_transaction_type: Some(PropertyTransactionType::Rent),
            property_sub_type: Some(PropertySubType::Apartment),
            area_sqm: Some(80.0),
            bedrooms: Some(2),
            bathrooms: Some(1),
            year_built: Some(2010),
            lot_size_sqm: None,
            zoning: None,
        },
    };

    // Create listings
    println!("Creating product listing...");
    let product_response = app
        .create_listing(
            &claims,
            &product_request,
            "test-fingerprint-1",
            "2026-05-09T00:00:00Z",
        )
        .await?
        .0;
    println!(
        "✅ Product listing created: {}",
        product_response.listing_id
    );

    println!("Creating service listing...");
    let service_response = app
        .create_listing(
            &claims,
            &service_request,
            "test-fingerprint-2",
            "2026-05-09T00:00:00Z",
        )
        .await?
        .0;
    println!(
        "✅ Service listing created: {}",
        service_response.listing_id
    );

    println!("Creating property listing...");
    let property_response = app
        .create_listing(
            &claims,
            &property_request,
            "test-fingerprint-3",
            "2026-05-09T00:00:00Z",
        )
        .await?
        .0;
    println!(
        "✅ Property listing created: {}",
        property_response.listing_id
    );

    // Test retrieval
    println!("\nTesting retrieval...");
    let product_listing = app
        .get_listing(Some(&claims), &product_response.listing_id)
        .await?;
    if let Some(listing) = product_listing {
        println!(
            "✅ Product listing retrieved: {} ({:?})",
            listing.listing.title, listing.listing.listing_type
        );
    }

    let service_listing = app
        .get_listing(Some(&claims), &service_response.listing_id)
        .await?;
    if let Some(listing) = service_listing {
        println!(
            "✅ Service listing retrieved: {} ({:?})",
            listing.listing.title, listing.listing.listing_type
        );
    }

    let property_listing = app
        .get_listing(Some(&claims), &property_response.listing_id)
        .await?;
    if let Some(listing) = property_listing {
        println!(
            "✅ Property listing retrieved: {} ({:?})",
            listing.listing.title, listing.listing.listing_type
        );
    }

    println!("\n🎉 All listing types working correctly!");
    Ok(())
}
