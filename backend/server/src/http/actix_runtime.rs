use crate::app::MarketplaceApp;
use crate::observability::ServerObservability;
use crate::repositories::audit_events::PostgresAuditEventRepository;
use crate::repositories::contact_reveals::PostgresContactRevealRepository;
use crate::repositories::listings::PostgresListingRepository;
use crate::repositories::outbox_events::PostgresOutboxEventRepository;
use crate::repositories::reservations::PostgresReservationLeaseRepository;
use crate::repositories::seller_accounts::PostgresSellerAccountRepository;
use crate::repositories::{AuditEventRepository, OutboxEventRepository, SellerAccountRepository};
use crate::services::idempotency::InMemoryIdempotencyRepository;
use actix_web::{web, App, HttpServer};
use moka::future::Cache;
use std::error::Error;
use std::sync::Arc;

// Production hardening: tracing
use tracing::{error, info};
use tracing_actix_web::TracingLogger;

// OpenAPI documentation
// use utoipa_swagger_ui::SwaggerUi; // Not used - using redirect to Swagger Editor
// use crate::openapi::ApiDoc; // Not needed - we serve JSON directly
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_run())
}

async fn async_run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let bind = std::env::var("MARKETPLACE_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    // Initialize tracing subscriber
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (pool, audit_repo, outbox_repo) = build_repositories().await?;
    let app = build_app(pool.clone(), audit_repo, outbox_repo);
    let observability = Arc::new(ServerObservability::new());

    let cache_enabled = std::env::var("MARKETPLACE_DISABLE_CACHE")
        .map(|value| value != "1" && !value.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    // Create Moka caches for Actix handlers (store pre-serialized JSON strings)
    let listing_cache: Cache<String, String> = Cache::new(10_000);
    let search_cache: Cache<String, String> = Cache::new(1_000);

    let app_data = web::Data::new(app);
    let obs_data = web::Data::new(observability);
    let cache_enabled_data = web::Data::new(cache_enabled);
    let listing_cache_data = web::Data::new(listing_cache);
    let search_cache_data = web::Data::new(search_cache);
    let pool_data = web::Data::new(pool);

    info!("Starting Actix-web server on {}", bind);

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default()) // Add tracing middleware
            .app_data(app_data.clone())
            .app_data(obs_data.clone())
            .app_data(cache_enabled_data.clone())
            .app_data(listing_cache_data.clone())
            .app_data(search_cache_data.clone())
            .app_data(pool_data.clone())
            // Redirect to Swagger Editor for interactive docs
            .route("/docs", web::get().to(crate::openapi::serve_swagger_editor))
            // Serve OpenAPI JSON
            .route(
                "/api-docs/openapi.json",
                web::get().to(crate::openapi::serve_openapi_json),
            )
            // Public API v1 routes - organized by resource type
            .service(
                // Product listings
                web::scope("/v1/product")
                    .route(
                        "/search",
                        web::get().to(crate::http::actix_handlers::search_listings),
                    )
                    .route(
                        "/{listing_id}",
                        web::get().to(crate::http::actix_handlers::get_listing),
                    )
                    .route(
                        "",
                        web::post().to(crate::http::actix_handlers::create_listing),
                    )
                    .route(
                        "/{listing_id}/reviews",
                        web::post().to(crate::http::actix_handlers::create_review),
                    )
                    .route(
                        "/{listing_id}/reviews",
                        web::get().to(crate::http::actix_handlers::list_reviews_for_listing),
                    ),
            )
            .service(
                // Service listings
                web::scope("/v1/service")
                    .route(
                        "/search",
                        web::get().to(crate::http::actix_handlers::search_listings),
                    )
                    .route(
                        "/{listing_id}",
                        web::get().to(crate::http::actix_handlers::get_listing),
                    )
                    .route(
                        "",
                        web::post().to(crate::http::actix_handlers::create_listing),
                    )
                    .route(
                        "/{listing_id}/reviews",
                        web::post().to(crate::http::actix_handlers::create_review),
                    )
                    .route(
                        "/{listing_id}/reviews",
                        web::get().to(crate::http::actix_handlers::list_reviews_for_listing),
                    ),
            )
            .service(
                // Property listings
                web::scope("/v1/property")
                    .route(
                        "/search",
                        web::get().to(crate::http::actix_handlers::search_listings),
                    )
                    .route(
                        "/{listing_id}",
                        web::get().to(crate::http::actix_handlers::get_listing),
                    )
                    .route(
                        "",
                        web::post().to(crate::http::actix_handlers::create_listing),
                    )
                    .route(
                        "/{listing_id}/reviews",
                        web::post().to(crate::http::actix_handlers::create_review),
                    )
                    .route(
                        "/{listing_id}/reviews",
                        web::get().to(crate::http::actix_handlers::list_reviews_for_listing),
                    ),
            )
            .service(
                // Shared resources (negotiations, contact reveals)
                web::scope("/v1")
                    .route(
                        "/negotiations",
                        web::post().to(crate::http::actix_handlers::open_negotiation),
                    )
                    .route(
                        "/contact-reveals",
                        web::post().to(crate::http::actix_handlers::request_contact_reveal),
                    ),
            )
            // Internal API v1 routes (admin/support)
            .service(
                web::scope("/internal/v1")
                    .route(
                        "/listings/{listing_id}/archive",
                        web::post().to(crate::http::actix_handlers::archive_listing),
                    )
                    .route(
                        "/reservations/{lease_id}/release",
                        web::post().to(crate::http::actix_handlers::release_reservation),
                    )
                    .route(
                        "/sellers/{seller_id}/trust-level",
                        web::put().to(crate::http::actix_handlers::set_seller_trust_level),
                    )
                    .route(
                        "/sellers/{seller_id}/quota-override",
                        web::put().to(crate::http::actix_handlers::set_seller_quota_override),
                    )
                    .route(
                        "/sellers/{seller_id}/recalculate-rating",
                        web::post().to(crate::http::actix_handlers::recalculate_seller_rating),
                    )
                    .route(
                        "/reviews/{review_id}/approve",
                        web::post().to(crate::http::actix_handlers::approve_review),
                    )
                    .route(
                        "/reviews/{review_id}/reject",
                        web::post().to(crate::http::actix_handlers::reject_review),
                    ),
            )
            // Metrics endpoint (simple version)
            .route("/metrics", web::get().to(metrics_handler))
            // Health check (deep)
            .route("/health", web::get().to(health_check))
    })
    .bind(&bind)?
    .run()
    .await?;

    Ok(())
}

async fn metrics_handler() -> impl actix_web::Responder {
    // Simple metrics endpoint - can be extended later
    let metrics =
        "# HELP requests_total Total requests\n# TYPE requests_total counter\nrequests_total 0\n"
            .to_string();
    actix_web::HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(metrics)
}

async fn health_check(pool: web::Data<sqlx::postgres::PgPool>) -> impl actix_web::Responder {
    let mut health = serde_json::json!({
        "status": "ok",
        "checks": {}
    });

    // Check database connectivity
    match sqlx::query("SELECT 1").execute(pool.get_ref()).await {
        Ok(_) => {
            health["checks"]["database"] = serde_json::json!({"status": "ok"});
        }
        Err(e) => {
            error!("Health check: DB error: {}", e);
            health["status"] = serde_json::json!("error");
            health["checks"]["database"] = serde_json::json!({
                "status": "error",
                "error": e.to_string()
            });
        }
    }

    // Check Moka cache (simple ping)
    health["checks"]["cache"] = serde_json::json!({"status": "ok"});

    let status = if health["status"] == "error" {
        actix_web::http::StatusCode::SERVICE_UNAVAILABLE
    } else {
        actix_web::http::StatusCode::OK
    };

    (actix_web::HttpResponse::Ok().json(health), status)
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
        .max_connections(20) // Increased for Actix's multi-worker model
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
) -> Arc<
    MarketplaceApp<
        PostgresListingRepository,
        InMemoryIdempotencyRepository,
        PostgresReservationLeaseRepository,
        PostgresContactRevealRepository,
    >,
> {
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
