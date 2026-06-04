use crate::app::MarketplaceApp;
use crate::domain::ledger::CreditLedgerRepository;
use crate::observability::ServerObservability;
use crate::repositories::audit_events::PostgresAuditEventRepository;
use crate::repositories::contact_reveals::PostgresContactRevealRepository;
use crate::repositories::ledger::PostgresCreditLedgerRepository;
use crate::repositories::listings::PostgresListingRepository;
use crate::repositories::negotiations::PostgresNegotiationRepository;
use crate::repositories::outbox_events::PostgresOutboxEventRepository;
use crate::repositories::reservations::PostgresReservationLeaseRepository;
use crate::repositories::seller_accounts::PostgresSellerAccountRepository;
use crate::repositories::{
    AuditEventRepository, OutboxEventRepository, PostgresIdempotencyKeyRepository,
    SellerAccountRepository,
};
use crate::services::agent_metrics::AgentMetricsCollector;
use crate::services::agent_registry::AgentRegistry;
use crate::services::async_committer::batch_channel;
use crate::services::circuit_breaker::CircuitBreakerRegistry;
use crate::services::ledger_cache::LedgerCache;
use crate::services::wal::WalManager;
use actix_web::{web, App, HttpServer};
use moka::future::Cache;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::broadcast;

// Production hardening: tracing
use tracing::{error, info};
use tracing_actix_web::TracingLogger;

// OpenAPI documentation
// use utoipa_swagger_ui::SwaggerUi; // Not used - using redirect to Swagger Editor
// use crate::openapi::ApiDoc; // Not needed - we serve JSON directly
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Configure tokio runtime for high concurrency performance
    let num_cpus = num_cpus::get();
    let max_worker_threads = 8; // Cap at 8 for database-focused workload
    let worker_threads = std::env::var("TOKIO_WORKER_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| num_cpus.saturating_sub(1).max(1).min(max_worker_threads));

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .thread_name("marketplace-worker")
        .thread_stack_size(2 * 1024 * 1024) // 2MB stack
        .enable_all()
        .build()?;

    runtime.block_on(async_run())
}

async fn async_run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let bind = std::env::var("MARKETPLACE_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    // Initialize tracing subscriber — LOG_FORMAT=json for production
    let log_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    if std::env::var("LOG_FORMAT").ok().as_deref() == Some("json") {
        tracing_subscriber::registry()
            .with(log_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(log_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    let (pool, audit_repo, outbox_repo) = build_repositories().await?;

    // Auto-run database migrations on startup
    info!("Running database schema migrations...");
    crate::bootstrap::apply_schema(&pool)
        .await
        .map_err(|e| std::io::Error::other(format!("migration failed: {e}")))?;
    info!("Schema migrations complete");

    let app = build_app(pool.clone(), audit_repo, outbox_repo);
    let observability = Arc::new(ServerObservability::new());

    let cache_enabled = std::env::var("MARKETPLACE_DISABLE_CACHE")
        .map(|value| value != "1" && !value.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    // Memory-based cache configuration (in MB)
    let listing_cache_max_mb: u64 = std::env::var("LISTING_CACHE_MAX_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200); // Default 200MB — ~67K listings at 3KB each
    let search_cache_max_mb: u64 = std::env::var("SEARCH_CACHE_MAX_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100); // Default 100MB — ~7K search results at 15KB each
    let listing_cache_max_bytes = listing_cache_max_mb * 1024 * 1024;
    let search_cache_max_bytes = search_cache_max_mb * 1024 * 1024;

    // Create Moka caches — weigher uses string byte length so max_capacity is total bytes
    let listing_cache: Cache<String, String> = Cache::builder()
        .weigher(|_key, value: &String| -> u32 { value.len() as u32 })
        .max_capacity(listing_cache_max_bytes)
        .time_to_live(std::time::Duration::from_secs(30 * 60)) // 30 minutes TTL
        .build();
    let search_cache: Cache<String, String> = Cache::builder()
        .weigher(|_key, value: &String| -> u32 { value.len() as u32 })
        .max_capacity(search_cache_max_bytes)
        .time_to_live(std::time::Duration::from_secs(15 * 60)) // 15 minutes TTL
        .build();

    info!(
        "Memory caches initialized: listing_cache={}MB, search_cache={}MB",
        listing_cache_max_mb, search_cache_max_mb
    );

    // Real-time event bus for SSE negotiation updates
    let (event_tx, _) = broadcast::channel::<String>(1024);
    let event_bus_data = web::Data::new(event_tx);

    // Credit ledger cache (write-through with TTL)
    let ledger_repo: Arc<dyn CreditLedgerRepository> =
        Arc::new(PostgresCreditLedgerRepository::new(pool.clone()));
    let ledger_cache = LedgerCache::new(ledger_repo.clone());
    let ledger_cache_data = web::Data::new(ledger_cache);

    // WAL recovery — replay any uncommitted transactions from a prior crash
    info!("Recovering credit ledger WAL...");
    let wal = WalManager::from_env()
        .map_err(|e| std::io::Error::other(format!("WAL initialization failed: {e}")))?;
    wal.recover(&*ledger_repo)
        .await
        .map_err(|e| std::io::Error::other(format!("WAL recovery failed: {e}")))?;
    info!("WAL recovery complete");

    // Async batch committer — background task for batching credit transactions
    let (batch_tx, batch_committer) = batch_channel(ledger_repo, Arc::new(wal));
    batch_committer.start();
    let batch_tx_data = web::Data::new(batch_tx);

    // Agent system — circuit breaker, registry, and metrics for agent routing
    let breaker_registry = web::Data::new(CircuitBreakerRegistry::default());
    let agent_registry = web::Data::new(AgentRegistry::default());
    let metrics_collector = web::Data::new(AgentMetricsCollector::default());

    let app_data = web::Data::new(app);
    let obs_data = web::Data::new(observability);
    let cache_enabled_data = web::Data::new(cache_enabled);
    let listing_cache_data = web::Data::new(listing_cache);
    let search_cache_data = web::Data::new(search_cache);
    let pool_data = web::Data::new(pool);
    let listing_cache_limit_data = web::Data::new(listing_cache_max_bytes);
    let search_cache_limit_data = web::Data::new(search_cache_max_bytes);

    info!("Starting Actix-web server on {}", bind);

    // Use more workers for high concurrency
    let num_cpus = num_cpus::get();
    let actix_workers = std::env::var("ACTIX_WORKERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| (num_cpus * 4).clamp(16, 64));
    let shutdown_timeout: u64 = std::env::var("SHUTDOWN_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Compress::default()) // Response compression (gzip)
            .wrap(TracingLogger::default()) // Add tracing middleware
            .app_data(app_data.clone())
            .app_data(obs_data.clone())
            .app_data(cache_enabled_data.clone())
            .app_data(listing_cache_data.clone())
            .app_data(search_cache_data.clone())
            .app_data(listing_cache_limit_data.clone())
            .app_data(search_cache_limit_data.clone())
            .app_data(ledger_cache_data.clone())
            .app_data(batch_tx_data.clone())
            .app_data(pool_data.clone())
            .app_data(event_bus_data.clone())
            .app_data(breaker_registry.clone())
            .app_data(agent_registry.clone())
            .app_data(metrics_collector.clone())
            // OpenAPI docs
            .route("/docs", web::get().to(crate::openapi::serve_swagger_editor))
            .route(
                "/api-docs/openapi.json",
                web::get().to(crate::openapi::serve_openapi_json),
            )
            // All API routes (listings, negotiations, reveals, internal)
            .configure(crate::http::actix_handlers::register_api_routes)
            // Metrics + health
            .route("/metrics", web::get().to(metrics_handler))
            .route("/health", web::get().to(health_check))
    })
    .bind(&bind)?
    .workers(actix_workers)
    .shutdown_timeout(shutdown_timeout)
    .run();

    // Handle graceful shutdown on SIGINT/SIGTERM
    let server_handle = server.handle();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutdown signal received, draining connections...");
        server_handle.stop(true).await;
        info!("Server stopped");
    });

    info!("Server ready — listening on {}", bind);
    server.await?;

    Ok(())
}

async fn metrics_handler(
    pool: web::Data<sqlx::postgres::PgPool>,
    listing_cache: web::Data<Cache<String, String>>,
    search_cache: web::Data<Cache<String, String>>,
    listing_max_bytes: web::Data<u64>,
    search_max_bytes: web::Data<u64>,
) -> impl actix_web::Responder {
    // Connection pool metrics
    let pool_size = pool.size();
    let idle_connections = pool.num_idle();

    // Runtime metrics
    let num_cpus = num_cpus::get();
    let max_worker_threads = 8usize; // Must match the cap in run()
    let worker_threads = std::env::var("TOKIO_WORKER_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| num_cpus.saturating_sub(1).max(1).min(max_worker_threads));

    // Cache metrics — derive size from the actual configured limits
    let listing_weight_capacity = **listing_max_bytes;
    let search_weight_capacity = **search_max_bytes;
    let listing_count = listing_cache.as_ref().entry_count();
    let search_count = search_cache.as_ref().entry_count();

    // Calculate utilization percentages
    let active_connections = pool_size.saturating_sub(idle_connections as u32);
    let connection_utilization = if pool_size > 0 {
        ((active_connections as f64 / pool_size as f64) * 100.0) as u32
    } else {
        0
    };

    // Estimate memory usage (avg 3KB per listing, 15KB per search result)
    let listing_memory_bytes = listing_count * 3 * 1024;
    let search_memory_bytes = search_count * 15 * 1024;

    let listing_cache_utilization = if listing_weight_capacity > 0 {
        ((listing_memory_bytes as f64 / listing_weight_capacity as f64) * 100.0) as u32
    } else {
        0
    };
    let search_cache_utilization = if search_weight_capacity > 0 {
        ((search_memory_bytes as f64 / search_weight_capacity as f64) * 100.0) as u32
    } else {
        0
    };

    // Memory usage in MB
    let listing_memory_mb = listing_memory_bytes / (1024 * 1024);
    let search_memory_mb = search_memory_bytes / (1024 * 1024);
    let total_cache_mb = listing_memory_mb + search_memory_mb;
    let listing_max_mb = listing_weight_capacity / (1024 * 1024);
    let search_max_mb = search_weight_capacity / (1024 * 1024);

    let metrics = format!(
        "# HELP database_connections_total Total database connections\n# TYPE database_connections_total gauge\ndatabase_connections_total {}\n\
         # HELP database_connections_idle Idle database connections\n# TYPE database_connections_idle gauge\ndatabase_connections_idle {}\n\
         # HELP database_connections_utilization_percent Connection pool utilization percentage\n# TYPE database_connections_utilization_percent gauge\ndatabase_connections_utilization_percent {}\n\
         # HELP runtime_worker_threads Configured tokio worker threads\n# TYPE runtime_worker_threads gauge\nruntime_worker_threads {}\n\
         # HELP runtime_max_worker_threads Max capped tokio worker threads\n# TYPE runtime_max_worker_threads gauge\nruntime_max_worker_threads {}\n\
         # HELP runtime_cpu_cores Available CPU cores\n# TYPE runtime_cpu_cores gauge\nruntime_cpu_cores {}\n\
         # HELP cache_listing_entries Current listing cache entries\n# TYPE cache_listing_entries gauge\ncache_listing_entries {}\n\
         # HELP cache_listing_memory_mb Current listing cache memory usage in MB\n# TYPE cache_listing_memory_mb gauge\ncache_listing_memory_mb {}\n\
         # HELP cache_listing_max_mb Listing cache max memory limit in MB\n# TYPE cache_listing_max_mb gauge\ncache_listing_max_mb {}\n\
         # HELP cache_listing_utilization_percent Listing cache utilization percentage\n# TYPE cache_listing_utilization_percent gauge\ncache_listing_utilization_percent {}\n\
         # HELP cache_search_entries Current search cache entries\n# TYPE cache_search_entries gauge\ncache_search_entries {}\n\
         # HELP cache_search_memory_mb Current search cache memory usage in MB\n# TYPE cache_search_memory_mb gauge\ncache_search_memory_mb {}\n\
         # HELP cache_search_max_mb Search cache max memory limit in MB\n# TYPE cache_search_max_mb gauge\ncache_search_max_mb {}\n\
         # HELP cache_search_utilization_percent Search cache utilization percentage\n# TYPE cache_search_utilization_percent gauge\ncache_search_utilization_percent {}\n\
         # HELP memory_cache_total_mb Total cache memory usage in MB\n# TYPE memory_cache_total_mb gauge\nmemory_cache_total_mb {}\n\
         # HELP requests_total Total requests\n# TYPE requests_total counter\nrequests_total 0\n",
        pool_size, idle_connections, connection_utilization, worker_threads, max_worker_threads, num_cpus,
        listing_count, listing_memory_mb, listing_max_mb, listing_cache_utilization,
        search_count, search_memory_mb, search_max_mb, search_cache_utilization,
        total_cache_mb
    );

    actix_web::HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(metrics)
}

async fn health_check(pool: web::Data<sqlx::postgres::PgPool>) -> impl actix_web::Responder {
    let mut health = serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
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
    let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(200); // Increased for high concurrency performance

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
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
        PostgresIdempotencyKeyRepository,
        PostgresReservationLeaseRepository,
        PostgresContactRevealRepository,
    >,
> {
    let listing_repository = PostgresListingRepository::new(pool.clone());
    let idempotency_repository = PostgresIdempotencyKeyRepository::new(pool.clone());
    let reservation_repository = PostgresReservationLeaseRepository::new(pool.clone());
    let contact_reveal_repository = PostgresContactRevealRepository::new(pool.clone());
    let negotiation_repository = Arc::new(PostgresNegotiationRepository::new(pool.clone()));
    let seller_account_repository: Arc<dyn SellerAccountRepository> =
        Arc::new(PostgresSellerAccountRepository::new(pool.clone()));

    let app = MarketplaceApp::new(
        listing_repository,
        idempotency_repository,
        reservation_repository,
        contact_reveal_repository,
        negotiation_repository,
        audit_repo,
        outbox_repo,
        seller_account_repository,
    );

    Arc::new(app)
}
