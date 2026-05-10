use async_trait::async_trait;
use marketplace_api_contract::{
    Category, Condition, CreateListingRequest, ListingLocation, ListingPayload,
    OpenNegotiationRequest, Price, RequestContactRevealRequest, SearchRequest, SearchSort,
    ShippingInfo,
};
use marketplace_auth_core::{Claims, Role, Scope};
use marketplace_server::app::MarketplaceApp;
use marketplace_server::repositories::audit_events::InMemoryAuditEventRepository;
use marketplace_server::repositories::audit_events::PostgresAuditEventRepository;
use marketplace_server::repositories::contact_reveals::InMemoryContactRevealRepository;
use marketplace_server::repositories::contact_reveals::PostgresContactRevealRepository;
use marketplace_server::repositories::listings::InMemoryListingRepository;
use marketplace_server::repositories::listings::PostgresListingRepository;
use marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository;
use marketplace_server::repositories::outbox_events::PostgresOutboxEventRepository;
use marketplace_server::repositories::reservations::InMemoryReservationLeaseRepository;
use marketplace_server::repositories::reservations::PostgresReservationLeaseRepository;
use marketplace_server::repositories::seller_accounts::InMemorySellerAccountRepository;
use marketplace_server::repositories::seller_accounts::PostgresSellerAccountRepository;
use marketplace_server::services::idempotency::InMemoryIdempotencyRepository;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::error::Error;
use std::time::Instant;

type PostgresBenchmarkApp = MarketplaceApp<
    PostgresListingRepository,
    InMemoryIdempotencyRepository,
    PostgresReservationLeaseRepository,
    PostgresContactRevealRepository,
>;

struct PostgresHarness {
    app: PostgresBenchmarkApp,
    pool: PgPool,
}

type MemoryBenchmarkApp = MarketplaceApp<
    InMemoryListingRepository,
    InMemoryIdempotencyRepository,
    InMemoryReservationLeaseRepository,
    InMemoryContactRevealRepository,
>;

#[derive(Clone, Copy)]
enum BenchmarkStep {
    Read,
    Search,
    OpenNegotiation,
    Reveal,
}

struct BenchmarkReport {
    name: &'static str,
    operations: usize,
    elapsed_ms: u128,
}

enum BenchmarkHarness {
    Postgres(PostgresHarness),
    Memory(MemoryBenchmarkApp),
}

#[async_trait]
trait BenchmarkAppFacade {
    async fn create_listing(
        &self,
        claims: &Claims,
        request: &CreateListingRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<marketplace_api_contract::CreateListingResponse, Box<dyn Error + Send + Sync>>;

    async fn get_listing(
        &self,
        claims: Option<&Claims>,
        listing_id: &str,
    ) -> Result<Option<marketplace_api_contract::ListingSummary>, Box<dyn Error + Send + Sync>>;

    async fn search_listings(
        &self,
        claims: Option<&Claims>,
        request: &SearchRequest,
    ) -> Result<marketplace_api_contract::SearchResponse, Box<dyn Error + Send + Sync>>;

    async fn open_negotiation(
        &self,
        claims: &Claims,
        request: &OpenNegotiationRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<marketplace_api_contract::NegotiationResponse, Box<dyn Error + Send + Sync>>;

    async fn request_contact_reveal(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &RequestContactRevealRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<marketplace_api_contract::ContactRevealResponse, Box<dyn Error + Send + Sync>>;

    async fn approve_contact_reveal(
        &self,
        claims: &Claims,
        reveal_id: &str,
    ) -> Result<marketplace_api_contract::ContactRevealResponse, Box<dyn Error + Send + Sync>>;

    fn get_pool(&self) -> Option<&sqlx::PgPool>;
}

#[async_trait]
impl BenchmarkAppFacade for BenchmarkHarness {
    async fn create_listing(
        &self,
        claims: &Claims,
        request: &CreateListingRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<marketplace_api_contract::CreateListingResponse, Box<dyn Error + Send + Sync>> {
        match self {
            Self::Postgres(harness) => Ok(harness
                .app
                .create_listing(claims, request, request_fingerprint, now_rfc3339)
                .await?),
            Self::Memory(app) => Ok(app
                .create_listing(claims, request, request_fingerprint, now_rfc3339)
                .await?),
        }
    }

    async fn get_listing(
        &self,
        claims: Option<&Claims>,
        listing_id: &str,
    ) -> Result<Option<marketplace_api_contract::ListingSummary>, Box<dyn Error + Send + Sync>>
    {
        match self {
            Self::Postgres(harness) => Ok(harness.app.get_listing(claims, listing_id).await?),
            Self::Memory(app) => Ok(app.get_listing(claims, listing_id).await?),
        }
    }

    async fn search_listings(
        &self,
        claims: Option<&Claims>,
        request: &SearchRequest,
    ) -> Result<marketplace_api_contract::SearchResponse, Box<dyn Error + Send + Sync>> {
        match self {
            Self::Postgres(harness) => Ok(harness.app.search_listings(claims, request).await?),
            Self::Memory(app) => Ok(app.search_listings(claims, request).await?),
        }
    }

    async fn open_negotiation(
        &self,
        claims: &Claims,
        request: &OpenNegotiationRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<marketplace_api_contract::NegotiationResponse, Box<dyn Error + Send + Sync>> {
        match self {
            Self::Postgres(harness) => Ok(harness
                .app
                .open_negotiation(claims, request, request_fingerprint, now_rfc3339)
                .await?),
            Self::Memory(app) => Ok(app
                .open_negotiation(claims, request, request_fingerprint, now_rfc3339)
                .await?),
        }
    }

    async fn request_contact_reveal(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &RequestContactRevealRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<marketplace_api_contract::ContactRevealResponse, Box<dyn Error + Send + Sync>> {
        match self {
            Self::Postgres(harness) => Ok(harness
                .app
                .request_contact_reveal(
                    claims,
                    negotiation_id,
                    request,
                    request_fingerprint,
                    now_rfc3339,
                )
                .await?),
            Self::Memory(app) => Ok(app
                .request_contact_reveal(
                    claims,
                    negotiation_id,
                    request,
                    request_fingerprint,
                    now_rfc3339,
                )
                .await?),
        }
    }

    async fn approve_contact_reveal(
        &self,
        claims: &Claims,
        reveal_id: &str,
    ) -> Result<marketplace_api_contract::ContactRevealResponse, Box<dyn Error + Send + Sync>> {
        match self {
            Self::Postgres(harness) => Ok(harness
                .app
                .approve_contact_reveal(claims, reveal_id)
                .await?),
            Self::Memory(app) => Ok(app.approve_contact_reveal(claims, reveal_id).await?),
        }
    }

    fn get_pool(&self) -> Option<&sqlx::PgPool> {
        match self {
            Self::Postgres(harness) => Some(&harness.pool),
            Self::Memory(_) => None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let harness = if let Ok(database_url) = std::env::var("DATABASE_URL") {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;
        BenchmarkHarness::Postgres(build_postgres_app(pool).await?)
    } else {
        eprintln!("DATABASE_URL is not set, using in-memory benchmark harness");
        BenchmarkHarness::Memory(build_memory_app())
    };
    let reader_claims = benchmark_reader_claims();
    let buyer_claims = benchmark_buyer_claims();
    let approver_claims = benchmark_approver_claims();
    let target_ops = benchmark_target_ops();
    let search_request = benchmark_search_request();
    let listing_read_pattern = benchmark_listing_read_pattern();
    let search_heavy_pattern = benchmark_search_heavy_pattern();
    let negotiation_burst_pattern = benchmark_negotiation_burst_pattern();

    let listing_ids = if let Some(pool) = harness.get_pool() {
        let listing_pool_size = benchmark_listing_pool_size(target_ops);
        load_listing_ids(pool, listing_pool_size).await?
    } else {
        let seller_claims = benchmark_seller_claims();
        seed_seller_accounts(&harness, &seller_claims).await?;
        let seed_count = benchmark_seed_count(target_ops);
        seed_listings(&harness, &seller_claims, seed_count).await?
    };

    if let Some(pool) = harness.get_pool() {
        clear_benchmark_transaction_tables(pool).await?;
    }

    eprintln!("Benchmark target: {target_ops} ops per profile");
    eprintln!("Benchmark listing pool: {}", listing_ids.len());

    let listing_read_report = run_profile(
        "listing-read",
        &harness,
        &listing_ids,
        &reader_claims,
        &buyer_claims,
        &approver_claims,
        &search_request,
        &listing_read_pattern,
        rounds_for_target_ops(&listing_read_pattern, target_ops),
        0,
        0,
    )
    .await?;
    if let Some(pool) = harness.get_pool() {
        clear_benchmark_transaction_tables(pool).await?;
    }

    let search_heavy_report = run_profile(
        "search-heavy",
        &harness,
        &listing_ids,
        &reader_claims,
        &buyer_claims,
        &approver_claims,
        &search_request,
        &search_heavy_pattern,
        rounds_for_target_ops(&search_heavy_pattern, target_ops),
        10,
        10,
    )
    .await?;
    if let Some(pool) = harness.get_pool() {
        clear_benchmark_transaction_tables(pool).await?;
    }

    let negotiation_burst_report = run_profile(
        "negotiation-burst",
        &harness,
        &listing_ids,
        &reader_claims,
        &buyer_claims,
        &approver_claims,
        &search_request,
        &negotiation_burst_pattern,
        rounds_for_target_ops(&negotiation_burst_pattern, target_ops),
        60,
        60,
    )
    .await?;

    let reports = [
        listing_read_report,
        search_heavy_report,
        negotiation_burst_report,
    ];

    println!("Phase 5 benchmark summary");
    println!("profile | ops | elapsed_ms | ops_per_sec");
    for report in reports {
        let ops_per_sec = if report.elapsed_ms == 0 {
            0.0
        } else {
            (report.operations as f64) / (report.elapsed_ms as f64 / 1000.0)
        };
        println!(
            "{} | {} | {} | {:.2}",
            report.name, report.operations, report.elapsed_ms, ops_per_sec
        );
    }

    Ok(())
}

fn build_memory_app() -> MemoryBenchmarkApp {
    MemoryBenchmarkApp::new(
        InMemoryListingRepository::new(),
        InMemoryIdempotencyRepository::new(),
        InMemoryReservationLeaseRepository::new(),
        InMemoryContactRevealRepository::new(),
        std::sync::Arc::new(InMemoryAuditEventRepository::new()),
        std::sync::Arc::new(InMemoryOutboxEventRepository::new()),
        std::sync::Arc::new(InMemorySellerAccountRepository::new()),
    )
}

async fn build_postgres_app(pool: PgPool) -> Result<PostgresHarness, Box<dyn Error + Send + Sync>> {
    let app = PostgresBenchmarkApp::new(
        PostgresListingRepository::new(pool.clone()),
        InMemoryIdempotencyRepository::new(),
        PostgresReservationLeaseRepository::new(pool.clone()),
        PostgresContactRevealRepository::new(pool.clone()),
        std::sync::Arc::new(PostgresAuditEventRepository::new(pool.clone())),
        std::sync::Arc::new(PostgresOutboxEventRepository::new(pool.clone())),
        std::sync::Arc::new(PostgresSellerAccountRepository::new(pool.clone())),
    );
    Ok(PostgresHarness { app, pool })
}

async fn clear_benchmark_transaction_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "TRUNCATE TABLE idempotency_keys, outbox_events, audit_events, contact_reveals, reservation_leases, negotiations",
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn benchmark_seller_claims() -> Claims {
    Claims {
        sub: "bench-seller-sub".to_string(),
        roles: vec![Role::SellerListingWriter, Role::SellerContactRevealApprover],
        scopes: vec![Scope::ListingCreate, Scope::RevealApprove],
        seller_account_id: Some("bench-seller".to_string()),
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    }
}

fn benchmark_reader_claims() -> Claims {
    Claims {
        sub: "bench-reader-sub".to_string(),
        roles: vec![Role::BuyerSearcher],
        scopes: vec![Scope::ListingRead, Scope::ListingSearch],
        seller_account_id: None,
        buyer_agent_id: Some("bench-buyer".to_string()),
        hardware_id: None,
        exp: None,
    }
}

fn benchmark_buyer_claims() -> Claims {
    Claims {
        sub: "bench-buyer-sub".to_string(),
        roles: vec![Role::BuyerNegotiator],
        scopes: vec![
            Scope::ListingRead,
            Scope::ListingSearch,
            Scope::NegotiationCreate,
            Scope::NegotiationRead,
            Scope::NegotiationOfferSubmit,
            Scope::NegotiationRevealRequest,
        ],
        seller_account_id: None,
        buyer_agent_id: Some("bench-buyer".to_string()),
        hardware_id: None,
        exp: None,
    }
}

fn benchmark_approver_claims() -> Claims {
    Claims {
        sub: "bench-approver-sub".to_string(),
        roles: vec![Role::SellerContactRevealApprover],
        scopes: vec![Scope::ListingRead, Scope::RevealApprove],
        seller_account_id: Some("bench-seller".to_string()),
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    }
}

fn benchmark_search_request() -> SearchRequest {
    use marketplace_api_contract::ListingType;

    SearchRequest {
        query: Some("Benchmark".to_string()), // Changed to match all listing titles
        category: Some(Category::Laptop),     // Only applies to Product listings
        condition: Some(Condition::Used),     // Only applies to Product listings
        listing_type: Some(ListingType::Product), // Test filtering by listing type
        sort_by: SearchSort::Relevance,
        limit: Some(20),
        ..SearchRequest::default()
    }
}

fn benchmark_listing_read_pattern() -> Vec<BenchmarkStep> {
    let mut steps = Vec::with_capacity(50);
    for _ in 0..4 {
        steps.extend(std::iter::repeat_n(BenchmarkStep::Read, 9));
        steps.push(BenchmarkStep::Search);
    }
    steps.extend(std::iter::repeat_n(BenchmarkStep::Read, 9));
    steps.push(BenchmarkStep::OpenNegotiation);
    steps
}

fn benchmark_search_heavy_pattern() -> Vec<BenchmarkStep> {
    let mut steps = Vec::with_capacity(50);
    for _ in 0..5 {
        steps.extend(std::iter::repeat_n(BenchmarkStep::Search, 6));
        steps.extend(std::iter::repeat_n(BenchmarkStep::Read, 3));
        steps.push(BenchmarkStep::OpenNegotiation);
    }
    steps
}

fn benchmark_negotiation_burst_pattern() -> Vec<BenchmarkStep> {
    let mut steps = Vec::with_capacity(50);
    for _ in 0..5 {
        steps.extend([
            BenchmarkStep::Read,
            BenchmarkStep::Read,
            BenchmarkStep::Search,
            BenchmarkStep::Search,
            BenchmarkStep::OpenNegotiation,
            BenchmarkStep::Reveal,
            BenchmarkStep::Read,
            BenchmarkStep::Read,
            BenchmarkStep::Search,
            BenchmarkStep::Search,
        ]);
    }
    steps
}

async fn seed_seller_accounts<A: BenchmarkAppFacade + Sync>(
    app: &A,
    _claims: &Claims,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // For now, we'll create seller accounts manually via SQL since the app doesn't have admin endpoints yet
    // In a real scenario, this would be done through the admin API
    if let Some(pool) = app.get_pool() {
        // Always create the seller account since benchmark clears all tables
        let result = sqlx::query(r#"INSERT INTO seller_accounts (seller_account_id, owner_id, trust_level, status, display_name, seller_rating, listings_created, quota_override) VALUES ('bench-seller', 'bench-seller', 'verified', 'active', 'Benchmark Seller', 4.5, 0, 5000)"#)
            .execute(pool)
            .await;

        if let Err(e) = result {
            eprintln!("Warning: Failed to seed seller account: {}", e);
            // Try alternative approach
            eprintln!("Attempting alternative seeding...");
            let _ = sqlx::query("INSERT INTO seller_accounts (seller_account_id, owner_id, trust_level, status, quota_override) VALUES ('bench-seller', 'bench-seller', 'verified', 'active', 5000)")
                .execute(pool)
                .await;
        }
    }
    Ok(())
}

async fn seed_listings<A: BenchmarkAppFacade + Sync>(
    app: &A,
    claims: &Claims,
    count: usize,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut listing_ids = Vec::with_capacity(count);
    for index in 0..count {
        let request = build_listing_request(index);
        let response = app
            .create_listing(
                claims,
                &request,
                &format!("seed-fingerprint-{index}"),
                "2026-05-05T00:00:00Z",
            )
            .await?;
        listing_ids.push(response.listing_id);
    }
    Ok(listing_ids)
}

fn build_listing_request(seed: usize) -> CreateListingRequest {
    use marketplace_api_contract::{
        ListingType, PropertySubType, PropertyTransactionType, ServiceType,
    };

    let city = match seed % 3 {
        0 => "Osaka",
        1 => "Tokyo",
        _ => "Kyoto",
    };

    // Create different types of listings based on seed
    let (
        listing_type,
        title,
        category,
        condition,
        service_type,
        hourly_rate,
        project_rate,
        qualifications,
        service_radius_km,
        property_transaction_type,
        property_sub_type,
        area_sqm,
        bedrooms,
        bathrooms,
        year_built,
        lot_size_sqm,
        zoning,
    ) = match seed % 3 {
        0 => {
            // Product listing (laptop)
            let thinkpad = !seed.is_multiple_of(5);
            let product_title = if thinkpad {
                format!("ThinkPad Benchmark {seed}")
            } else {
                format!("Latitude Benchmark {seed}")
            };
            (
                ListingType::Product,
                product_title,
                Some(Category::Laptop),
                Some(Condition::Used),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }
        1 => {
            // Service listing (tutoring)
            (
                ListingType::Service,
                format!("Math Tutoring Service {seed}"),
                None,
                None,
                Some(ServiceType::Local),
                Some(25.0 + (seed % 5) as f64),
                Some(200.0 + (seed % 10) as f64),
                Some(vec![
                    "Teaching License".to_string(),
                    "Math Degree".to_string(),
                ]),
                Some(10 + (seed % 5) as i32),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }
        _ => {
            // Property listing (apartment for rent)
            (
                ListingType::Property,
                format!("Apartment for Rent {seed}"),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(PropertyTransactionType::Rent),
                Some(PropertySubType::Apartment),
                Some(80.0 + (seed % 20) as f64),
                Some(2 + (seed % 3) as i32),
                Some(1 + (seed % 2) as i32),
                Some(2010 + (seed % 15) as i32),
                None,
                None,
            )
        }
    };

    CreateListingRequest {
        idempotency_key: format!("seed-idem-{seed}"),
        listing: ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "bench-seller".to_string(),
            listing_type,
            category,
            title,
            condition,
            price: Price {
                currency: "USD".to_string(),
                amount: match listing_type {
                    ListingType::Product => 499.0 + seed as f64,
                    ListingType::Service => hourly_rate.unwrap_or(25.0),
                    ListingType::Property => 1200.0 + (seed % 5 * 200) as f64,
                },
            },
            location: ListingLocation {
                country_code: "JP".to_string(),
                country_name: "Japan".to_string(),
                city: city.to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec!["https://example.com/item.jpg".to_string()],
            description: format!("Benchmark listing {seed} ({:?})", listing_type),
            attributes: Some(serde_json::json!({
                "seed": seed,
                "listing_type": format!("{:?}", listing_type),
            })),
            // Marketplace fields
            sku: if matches!(listing_type, ListingType::Product) {
                Some(format!("SKU-{seed}"))
            } else {
                None
            },
            quantity: if matches!(listing_type, ListingType::Product) {
                Some(1)
            } else {
                None
            },
            shipping_info: if matches!(listing_type, ListingType::Product) {
                Some(ShippingInfo {
                    local_pickup: true,
                    shipping_available: true,
                    shipping_cost: Some(Price {
                        currency: "USD".to_string(),
                        amount: 15.99,
                    }),
                    shipping_regions: Some(vec!["US".to_string(), "CA".to_string()]),
                })
            } else {
                None
            },
            condition_details: if matches!(listing_type, ListingType::Product) {
                Some("Excellent condition, lightly used".to_string())
            } else {
                None
            },
            seller_notes: if seed.is_multiple_of(10) {
                Some("Special discount available!".to_string())
            } else {
                None
            },
            // Service fields
            service_type,
            hourly_rate,
            project_rate,
            qualifications,
            service_radius_km,
            // Property fields
            property_transaction_type,
            property_sub_type,
            area_sqm,
            bedrooms,
            bathrooms,
            year_built,
            lot_size_sqm,
            zoning,
        },
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_profile<A: BenchmarkAppFacade + Sync>(
    name: &'static str,
    app: &A,
    listing_ids: &[String],
    reader_claims: &Claims,
    buyer_claims: &Claims,
    approver_claims: &Claims,
    search_request: &SearchRequest,
    pattern: &[BenchmarkStep],
    rounds: usize,
    read_start: usize,
    negotiation_start: usize,
) -> Result<BenchmarkReport, Box<dyn Error + Send + Sync>> {
    let start = Instant::now();
    let mut listing_cursor = read_start;
    let mut negotiation_cursor = negotiation_start;
    let mut step_count = 0usize;
    let mut operation_count = 0usize;

    for round in 0..rounds {
        for step in pattern {
            match step {
                BenchmarkStep::Read => {
                    let listing_id = next_listing_id(listing_ids, &mut listing_cursor);
                    let _ = app.get_listing(Some(reader_claims), &listing_id).await?;
                    operation_count += 1;
                }
                BenchmarkStep::Search => {
                    let _ = app
                        .search_listings(Some(reader_claims), search_request)
                        .await?;
                    operation_count += 1;
                }
                BenchmarkStep::OpenNegotiation => {
                    let listing_id = next_listing_id(listing_ids, &mut negotiation_cursor);
                    let request = OpenNegotiationRequest {
                        listing_id: listing_id.clone(),
                        buyer_agent_id: "bench-buyer".to_string(),
                        offer_currency: "USD".to_string(),
                        offer_amount: 499.0,
                        idempotency_key: format!("bench-open-{name}-{round}-{step_count}"),
                    };
                    let _ = app
                        .open_negotiation(
                            buyer_claims,
                            &request,
                            &format!("fp-open-{name}-{round}-{step_count}"),
                            "2026-05-05T00:00:00Z",
                        )
                        .await?;
                    operation_count += 1;
                }
                BenchmarkStep::Reveal => {
                    let listing_id = next_listing_id(listing_ids, &mut negotiation_cursor);
                    let request = OpenNegotiationRequest {
                        listing_id: listing_id.clone(),
                        buyer_agent_id: "bench-buyer".to_string(),
                        offer_currency: "USD".to_string(),
                        offer_amount: 499.0,
                        idempotency_key: format!("bench-reveal-open-{name}-{round}-{step_count}"),
                    };
                    let negotiation = app
                        .open_negotiation(
                            buyer_claims,
                            &request,
                            &format!("fp-reveal-open-{name}-{round}-{step_count}"),
                            "2026-05-05T00:00:00Z",
                        )
                        .await?;
                    let reveal = app
                        .request_contact_reveal(
                            buyer_claims,
                            &negotiation.negotiation_id,
                            &RequestContactRevealRequest {
                                idempotency_key: format!(
                                    "bench-reveal-request-{name}-{round}-{step_count}"
                                ),
                            },
                            &format!("fp-reveal-request-{name}-{round}-{step_count}"),
                            "2026-05-05T00:00:00Z",
                        )
                        .await?;
                    let _ = app
                        .approve_contact_reveal(approver_claims, &reveal.reveal_id)
                        .await?;
                    operation_count += 3;
                }
            }
            step_count += 1;
        }
    }

    let elapsed_ms = start.elapsed().as_millis();

    Ok(BenchmarkReport {
        name,
        operations: operation_count,
        elapsed_ms,
    })
}

fn next_listing_id(listing_ids: &[String], cursor: &mut usize) -> String {
    let listing_id = listing_ids[*cursor % listing_ids.len()].clone();
    *cursor += 1;
    listing_id
}

fn benchmark_target_ops() -> usize {
    std::env::var("PHASE5_BENCH_OPS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(10_000)
}

fn benchmark_seed_count(target_ops: usize) -> usize {
    target_ops.div_ceil(3) + 100
}

fn benchmark_listing_pool_size(target_ops: usize) -> usize {
    benchmark_seed_count(target_ops).max(2_000)
}

async fn load_listing_ids(pool: &PgPool, limit: usize) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT l.listing_id
         FROM listings l
         LEFT JOIN reservation_leases rl
           ON rl.listing_id = l.listing_id AND rl.status = 'active'
         WHERE l.listing_type = 'product'
           AND rl.listing_id IS NULL
         ORDER BY l.listing_id
         LIMIT $1",
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Err(sqlx::Error::RowNotFound);
    }

    Ok(rows)
}

fn rounds_for_target_ops(pattern: &[BenchmarkStep], target_ops: usize) -> usize {
    let ops_per_round: usize = pattern
        .iter()
        .map(|step| match step {
            BenchmarkStep::Read | BenchmarkStep::Search | BenchmarkStep::OpenNegotiation => 1,
            BenchmarkStep::Reveal => 3,
        })
        .sum();

    target_ops.div_ceil(ops_per_round)
}
