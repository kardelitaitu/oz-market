use actix_web::test::{self, TestRequest};
use actix_web::web::{self, Data};
use actix_web::App;
use oz_market_api_contract::{
    Category, Condition, CreateListingRequest, ListingLocation, ListingPayload, Price,
};
use oz_market_server::app::MarketplaceApp;
use oz_market_server::repositories::audit_events::PostgresAuditEventRepository;
use oz_market_server::repositories::contact_reveals::PostgresContactRevealRepository;
use oz_market_server::repositories::listings::PostgresListingRepository;
use oz_market_server::repositories::negotiations::PostgresNegotiationRepository;
use oz_market_server::repositories::outbox_events::PostgresOutboxEventRepository;
use oz_market_server::repositories::reservations::PostgresReservationLeaseRepository;
use oz_market_server::repositories::seller_accounts::PostgresSellerAccountRepository;
use oz_market_server::repositories::{
    AuditEventRepository, OutboxEventRepository, PostgresIdempotencyKeyRepository,
    SellerAccountRepository,
};
use serde_json::json;
use sqlx::PgPool;
use std::error::Error;
use std::sync::Arc;

type E2eApp = MarketplaceApp<
    PostgresListingRepository,
    PostgresIdempotencyKeyRepository,
    PostgresReservationLeaseRepository,
    PostgresContactRevealRepository,
>;

fn create_listing_request() -> CreateListingRequest {
    CreateListingRequest {
        idempotency_key: "idem-create-e2e-1".to_string(),
        listing: ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "seller-1".to_string(),
            listing_type: oz_market_api_contract::ListingType::Product,
            category: Some(Category::Laptop),
            title: "E2E Product".to_string(),
            condition: Some(Condition::Used),
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
            price: Price {
                currency: "USD".to_string(),
                amount: 450.0,
            },
            location: ListingLocation {
                country_code: "JP".to_string(),
                country_name: "Japan".to_string(),
                city: "Osaka".to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec!["https://example.com/item.jpg".to_string()],
            description: "Good battery health".to_string(),
            attributes: None,
            sku: None,
            quantity: None,
            shipping_info: None,
            condition_details: None,
            seller_notes: None,
        },
    }
}

fn seller_claims_header() -> String {
    r#"{"sub":"sub-1","roles":["seller_listing_writer","buyer_negotiator","seller_contact_reveal_approver"],"scopes":["listing:create","listing:read","listing:search","negotiation:create","negotiation:read","negotiation:offer:submit","negotiation:reveal:request","reveal:approve"],"seller_account_id":"seller-1","buyer_agent_id":"buyer-1"}"#.to_string()
}

fn setup_app(pool: PgPool) -> Arc<E2eApp> {
    let listing_repo = PostgresListingRepository::new(pool.clone());
    let idempotency_repo = PostgresIdempotencyKeyRepository::new(pool.clone());
    let reservation_repo = PostgresReservationLeaseRepository::new(pool.clone());
    let contact_reveal_repo = PostgresContactRevealRepository::new(pool.clone());
    let negotiation_repo = Arc::new(PostgresNegotiationRepository::new(pool.clone()));
    let audit_repo: Arc<dyn AuditEventRepository> =
        Arc::new(PostgresAuditEventRepository::new(pool.clone()));
    let outbox_repo: Arc<dyn OutboxEventRepository> =
        Arc::new(PostgresOutboxEventRepository::new(pool.clone()));
    let seller_account_repo: Arc<dyn SellerAccountRepository> =
        Arc::new(PostgresSellerAccountRepository::new(pool));

    Arc::new(MarketplaceApp::new(
        listing_repo,
        idempotency_repo,
        reservation_repo,
        contact_reveal_repo,
        negotiation_repo,
        audit_repo,
        outbox_repo,
        seller_account_repo,
    ))
}

#[actix_web::test]
#[ignore = "requires a configured live Postgres DATABASE_URL"]
async fn e2e_create_listing_open_negotiation_and_request_reveal(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };
    let pool = PgPool::connect(&database_url).await?;
    let app = setup_app(pool);

    let app_data = Data::new(app);
    let server = test::init_service(
        App::new()
            .app_data(app_data.clone())
            .app_data(Data::new(false))
            .app_data(Data::new(moka::future::Cache::<String, String>::new(100)))
            .app_data(Data::new(moka::future::Cache::<String, String>::new(100)))
            .service(web::scope("/v1/listings").route(
                "",
                web::post().to(oz_market_server::http::actix_handlers::create_listing),
            ))
            .service(
                web::scope("/v1")
                    .route(
                        "/negotiations",
                        web::post().to(oz_market_server::http::actix_handlers::open_negotiation),
                    )
                    .route(
                        "/negotiations/{negotiation_id}/request-contact-reveal",
                        web::post()
                            .to(oz_market_server::http::actix_handlers::request_contact_reveal),
                    ),
            ),
    )
    .await;

    let create_req = TestRequest::post()
        .uri("/v1/listings")
        .insert_header(("x-marketplace-claims", seller_claims_header()))
        .set_json(create_listing_request())
        .to_request();
    let create_resp = test::call_service(&server, create_req).await;
    assert!(
        create_resp.status() == 200 || create_resp.status() == 201,
        "create listing: expected 200 or 201, got {}",
        create_resp.status()
    );
    let create_body: serde_json::Value = test::read_body_json(create_resp).await;
    let listing_id = create_body["listing_id"].as_str().unwrap().to_string();

    let open_req = TestRequest::post()
        .uri("/v1/negotiations")
        .insert_header(("x-marketplace-claims", seller_claims_header()))
        .set_json(json!({
            "listing_id": listing_id,
            "buyer_agent_id": "buyer-1",
            "offer_amount": 440.0,
            "offer_currency": "USD",
            "idempotency_key": "idem-open-e2e-1"
        }))
        .to_request();
    let open_resp = test::call_service(&server, open_req).await;
    assert!(
        open_resp.status() == 200 || open_resp.status() == 201,
        "open negotiation: expected 200 or 201, got {}",
        open_resp.status()
    );
    let open_body: serde_json::Value = test::read_body_json(open_resp).await;
    let negotiation_id = open_body["negotiation_id"].as_str().unwrap().to_string();

    let reveal_req = TestRequest::post()
        .uri(&format!(
            "/v1/negotiations/{}/request-contact-reveal",
            negotiation_id
        ))
        .insert_header(("x-marketplace-claims", seller_claims_header()))
        .set_json(json!({
            "idempotency_key": "idem-reveal-e2e-1"
        }))
        .to_request();
    let reveal_resp = test::call_service(&server, reveal_req).await;
    assert_eq!(reveal_resp.status(), 202);

    Ok(())
}
