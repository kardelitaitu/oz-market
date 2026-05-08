use async_trait::async_trait;
use marketplace_api_contract::{
    Category, Condition, CreateListingRequest, ListingLocation, ListingPayload,
    OpenNegotiationRequest, Price, RequestContactRevealRequest, SearchRequest, SearchSort,
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
    Postgres(PostgresBenchmarkApp),
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
        claims: &Claims,
        listing_id: &str,
    ) -> Result<Option<marketplace_api_contract::ListingSummary>, Box<dyn Error + Send + Sync>>;

    async fn search_listings(
        &self,
        claims: &Claims,
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
            Self::Postgres(app) => Ok(app
                .create_listing(claims, request, request_fingerprint, now_rfc3339)
                .await?),
            Self::Memory(app) => Ok(app
                .create_listing(claims, request, request_fingerprint, now_rfc3339)
                .await?),
        }
    }

    async fn get_listing(
        &self,
        claims: &Claims,
        listing_id: &str,
    ) -> Result<Option<marketplace_api_contract::ListingSummary>, Box<dyn Error + Send + Sync>>
    {
        match self {
            Self::Postgres(app) => Ok(app.get_listing(claims, listing_id).await?),
            Self::Memory(app) => Ok(app.get_listing(claims, listing_id).await?),
        }
    }

    async fn search_listings(
        &self,
        claims: &Claims,
        request: &SearchRequest,
    ) -> Result<marketplace_api_contract::SearchResponse, Box<dyn Error + Send + Sync>> {
        match self {
            Self::Postgres(app) => Ok(app.search_listings(claims, request).await?),
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
            Self::Postgres(app) => Ok(app
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
            Self::Postgres(app) => Ok(app
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
            Self::Postgres(app) => Ok(app.approve_contact_reveal(claims, reveal_id).await?),
            Self::Memory(app) => Ok(app.approve_contact_reveal(claims, reveal_id).await?),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let harness = if let Some(database_url) = std::env::var("DATABASE_URL").ok() {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;
        ensure_schema(&pool).await?;
        clear_benchmark_tables(&pool).await?;
        BenchmarkHarness::Postgres(build_postgres_app(pool).await?)
    } else {
        eprintln!("DATABASE_URL is not set, using in-memory benchmark harness");
        BenchmarkHarness::Memory(build_memory_app())
    };
    let seller_claims = benchmark_seller_claims();
    let reader_claims = benchmark_reader_claims();
    let buyer_claims = benchmark_buyer_claims();
    let approver_claims = benchmark_approver_claims();

    let listing_ids = seed_listings(&harness, &seller_claims, 160).await?;
    let search_request = benchmark_search_request();
    let listing_read_pattern = benchmark_listing_read_pattern();
    let search_heavy_pattern = benchmark_search_heavy_pattern();
    let negotiation_burst_pattern = benchmark_negotiation_burst_pattern();

    let reports = [
        run_profile(
            "listing-read",
            &harness,
            &listing_ids,
            &reader_claims,
            &buyer_claims,
            &approver_claims,
            &search_request,
            &listing_read_pattern,
            10,
            0,
            0,
        )
        .await?,
        run_profile(
            "search-heavy",
            &harness,
            &listing_ids,
            &reader_claims,
            &buyer_claims,
            &approver_claims,
            &search_request,
            &search_heavy_pattern,
            10,
            10,
            10,
        )
        .await?,
        run_profile(
            "negotiation-burst",
            &harness,
            &listing_ids,
            &reader_claims,
            &buyer_claims,
            &approver_claims,
            &search_request,
            &negotiation_burst_pattern,
            5,
            60,
            60,
        )
        .await?,
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

async fn build_postgres_app(
    pool: PgPool,
) -> Result<PostgresBenchmarkApp, Box<dyn Error + Send + Sync>> {
    Ok(PostgresBenchmarkApp::new(
        PostgresListingRepository::new(pool.clone()),
        InMemoryIdempotencyRepository::new(),
        PostgresReservationLeaseRepository::new(pool.clone()),
        PostgresContactRevealRepository::new(pool.clone()),
        std::sync::Arc::new(PostgresAuditEventRepository::new(pool.clone())),
        std::sync::Arc::new(PostgresOutboxEventRepository::new(pool.clone())),
        std::sync::Arc::new(PostgresSellerAccountRepository::new(pool)),
    ))
}

async fn ensure_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    for statement in include_str!("../../migrations/0001_init.sql").split(';') {
        let statement = statement.trim();
        if statement.is_empty()
            || statement.eq_ignore_ascii_case("BEGIN")
            || statement.eq_ignore_ascii_case("COMMIT")
        {
            continue;
        }
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}

async fn clear_benchmark_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
    for statement in [
        "TRUNCATE TABLE idempotency_keys, outbox_events, audit_events, contact_reveals, reservation_leases, negotiations, listings RESTART IDENTITY CASCADE",
        "SELECT setval('listing_id_seq', 1, false)",
        "SELECT setval('reservation_lease_id_seq', 1, false)",
        "SELECT setval('contact_reveal_id_seq', 1, false)",
    ] {
        sqlx::query(statement).execute(pool).await?;
    }
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
    SearchRequest {
        query: Some("ThinkPad".to_string()),
        category: Some(Category::Laptop),
        condition: Some(Condition::Used),
        sort_by: SearchSort::Relevance,
        limit: Some(20),
        ..SearchRequest::default()
    }
}

fn benchmark_listing_read_pattern() -> Vec<BenchmarkStep> {
    let mut steps = Vec::with_capacity(50);
    for _ in 0..4 {
        steps.extend(std::iter::repeat(BenchmarkStep::Read).take(9));
        steps.push(BenchmarkStep::Search);
    }
    steps.extend(std::iter::repeat(BenchmarkStep::Read).take(9));
    steps.push(BenchmarkStep::OpenNegotiation);
    steps
}

fn benchmark_search_heavy_pattern() -> Vec<BenchmarkStep> {
    let mut steps = Vec::with_capacity(50);
    for _ in 0..5 {
        steps.extend(std::iter::repeat(BenchmarkStep::Search).take(6));
        steps.extend(std::iter::repeat(BenchmarkStep::Read).take(3));
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
    let thinkpad = seed % 5 != 0;
    let product_name = if thinkpad {
        format!("ThinkPad Benchmark {seed}")
    } else {
        format!("Latitude Benchmark {seed}")
    };
    let city = match seed % 3 {
        0 => "Osaka",
        1 => "Tokyo",
        _ => "Kyoto",
    };

    CreateListingRequest {
        idempotency_key: format!("seed-idem-{seed}"),
        listing: ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "bench-seller".to_string(),
            category: Category::Laptop,
            product_name,
            condition: Condition::Used,
            price: Price {
                currency: "USD".to_string(),
                amount: 499.0 + seed as f64,
            },
            location: ListingLocation {
                country_code: "JP".to_string(),
                country_name: "Japan".to_string(),
                city: city.to_string(),
                // Phase D: Geolocation (optional)
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec!["https://example.com/item.jpg".to_string()],
            description: format!("Benchmark listing {seed}"),
            attributes: Some(serde_json::json!({
                "brand": "Lenovo",
                "seed": seed,
            })),
            // NEW: Marketplace fields
            sku: None,
            quantity: None,
            shipping_info: None,
            condition_details: None,
            seller_notes: None,
        },
    }
}

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
                    let _ = app.get_listing(reader_claims, &listing_id).await?;
                    operation_count += 1;
                }
                BenchmarkStep::Search => {
                    let _ = app.search_listings(reader_claims, search_request).await?;
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
