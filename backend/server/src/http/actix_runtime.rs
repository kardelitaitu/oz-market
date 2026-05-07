use actix_web::{web, App, HttpServer, FromRequest, HttpRequest, HttpResponse};
use actix_web::dev::{ServiceRequest, Service};
use actix_web::Error as ActixError;
use crate::app::MarketplaceApp;
use crate::observability::ServerObservability;
use crate::repositories::{
    AuditEventRepository, OutboxEventRepository, SellerAccountRepository,
};
use crate::repositories::audit_events::PostgresAuditEventRepository;
use marketplace_auth_core::Claims;
use crate::repositories::contact_reveals::PostgresContactRevealRepository;
use crate::repositories::listings::PostgresListingRepository;
use crate::repositories::outbox_events::PostgresOutboxEventRepository;
use crate::repositories::reservations::PostgresReservationLeaseRepository;
use crate::repositories::seller_accounts::PostgresSellerAccountRepository;
use crate::services::idempotency::InMemoryIdempotencyRepository;
use moka::future::Cache;
use marketplace_api_contract::{SearchResponse, ListingSummary};
use std::error::Error;
use std::sync::Arc;

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_run())
}

async fn async_run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let bind = std::env::var("MARKETPLACE_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    
    let (pool, audit_repo, outbox_repo) = build_repositories().await?;
    let app = build_app(pool, audit_repo, outbox_repo);
    let observability = Arc::new(ServerObservability::new());
    
    // Create Moka caches for Actix handlers
    let listing_cache: Cache<String, ListingSummary> = Cache::new(10_000);
    let search_cache: Cache<String, SearchResponse> = Cache::new(1_000);
    
    let app_data = web::Data::new(app);
    let obs_data = web::Data::new(observability);
    let listing_cache_data = web::Data::new(listing_cache);
    let search_cache_data = web::Data::new(search_cache);
    
    println!("Starting Actix-web server on {}", bind);
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .app_data(obs_data.clone())
            .app_data(listing_cache_data.clone())
            .app_data(search_cache_data.clone())
            // Public API v1 routes
            .service(
                web::scope("/v1")
                    .route("/listings/search", web::get().to(crate::http::actix_handlers::search_listings))
                    .route("/listings/{listing_id}", web::get().to(crate::http::actix_handlers::get_listing))
                    .route("/listings", web::post().to(crate::http::actix_handlers::create_listing))
                    .route("/negotiations", web::post().to(crate::http::actix_handlers::open_negotiation))
                    .route("/contact-reveals", web::post().to(crate::http::actix_handlers::request_contact_reveal))
            )
            // Internal API v1 routes (admin/support)
            .service(
                web::scope("/internal/v1")
                    .route("/listings/{listing_id}/archive", web::post().to(crate::http::actix_handlers::archive_listing))
                    .route("/reservations/{lease_id}/release", web::post().to(crate::http::actix_handlers::release_reservation))
                    .route("/sellers/{seller_id}/trust-level", web::put().to(crate::http::actix_handlers::set_seller_trust_level))
                    .route("/sellers/{seller_id}/quota-override", web::put().to(crate::http::actix_handlers::set_seller_quota_override))
            )
            // Health check
            .route("/health", web::get().to(health_check))
    })
    .bind(&bind)?
    .run()
    .await?;
    
    Ok(())
}

async fn health_check() -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn build_repositories() -> Result<
    (
        sqlx::postgres::PgPool,
        Arc<dyn AuditEventRepository>,
        Arc<dyn OutboxEventRepository>,
    ),
    Box<dyn Error + Send + Sync>,
> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "DATABASE_URL is required for production runtime",
        )
    })?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)  // Increased for Actix's multi-worker model
        .connect(&database_url)
        .await?;
    let audit_repo: Arc<dyn AuditEventRepository> =
        Arc::new(PostgresAuditEventRepository::new(pool.clone()));
    let outbox_repo: Arc<dyn OutboxEventRepository> =
        Arc::new(PostgresOutboxEventRepository::new(pool.clone()));
    Ok((pool, audit_repo, outbox_repo))
}

fn build_app(
    pool: sqlx::postgres::PgPool,
    audit_repo: Arc<dyn AuditEventRepository>,
    outbox_repo: Arc<dyn OutboxEventRepository>,
) -> Arc<MarketplaceApp<
    PostgresListingRepository,
    InMemoryIdempotencyRepository,
    PostgresReservationLeaseRepository,
    PostgresContactRevealRepository,
>> {
    let listing_repository = PostgresListingRepository::new(pool.clone());
    let idempotency_repository = InMemoryIdempotencyRepository::new();
    let reservation_repository = PostgresReservationLeaseRepository::new(pool.clone());
    let contact_reveal_repository = PostgresContactRevealRepository::new(pool.clone());
    let seller_account_repository: Arc<dyn SellerAccountRepository> =
        Arc::new(PostgresSellerAccountRepository::new(pool.clone()));
    
    let app = MarketplaceApp::new(
        listing_repository,
        idempotency_repository,
        reservation_repository,
        contact_reveal_repository,
        audit_repo,
        outbox_repo,
        seller_account_repository,
    );
    
    Arc::new(app)
}
