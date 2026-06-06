use crate::app::MarketplaceApp;
use crate::domain::ledger::CreditLedgerRepository;
use crate::observability::{render_http_counter_metrics, ServerObservability};
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
use crate::services::agent_dispatcher::{AgentDispatcher, HttpAgentDispatcher};
use crate::services::agent_metrics::AgentMetricsCollector;
use crate::services::agent_registry::AgentRegistry;
use crate::services::async_committer::batch_channel;
use crate::services::circuit_breaker::CircuitBreakerRegistry;
use crate::services::ledger_cache::LedgerCache;
use crate::services::wal::WalManager;
use actix_web::{dev::Service, web, App, HttpServer};
use moka::future::Cache;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::broadcast;

// Production hardening: tracing
use tracing::{error, info};
use tracing_actix_web::TracingLogger;

type AgentSystemDeps = (
    web::Data<CircuitBreakerRegistry>,
    web::Data<AgentRegistry>,
    web::Data<AgentMetricsCollector>,
    web::Data<Arc<dyn AgentDispatcher>>,
);

// OpenAPI documentation
// use utoipa_swagger_ui::SwaggerUi; // Not used - using redirect to Swagger Editor
// use crate::openapi::ApiDoc; // Not needed - we serve JSON directly
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let worker_threads = resolve_worker_threads();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .thread_name("oz-market-worker")
        .thread_stack_size(2 * 1024 * 1024) // 2MB stack
        .enable_all()
        .build()?;

    runtime.block_on(async_run())
}

async fn async_run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let bind = resolve_bind_address();
    init_tracing();

    let (pool, audit_repo, outbox_repo) = build_repositories().await?;
    run_migrations(&pool).await?;

    let app = build_app(pool.clone(), audit_repo, outbox_repo);
    let observability = Arc::new(ServerObservability::new());

    let (listing_cache, search_cache, listing_cache_max_bytes, search_cache_max_bytes) =
        build_moka_caches();
    let event_bus_data = init_event_bus();

    let (ledger_cache_data, batch_tx_data) =
        init_ledger_system(pool.clone(), observability.clone()).await?;

    let (breaker_registry, agent_registry, metrics_collector, dispatcher_data) =
        init_agent_system();

    let (actix_workers, shutdown_timeout) = resolve_server_config();

    let deps = AppDependencies {
        app: web::Data::new(app),
        observability: web::Data::new(observability),
        cache_enabled: web::Data::new(cache_enabled()),
        listing_cache: web::Data::new(listing_cache),
        search_cache: web::Data::new(search_cache),
        listing_cache_limit: web::Data::new(listing_cache_max_bytes),
        search_cache_limit: web::Data::new(search_cache_max_bytes),
        ledger_cache: ledger_cache_data,
        batch_tx: batch_tx_data,
        event_bus: event_bus_data,
        breaker_registry,
        agent_registry,
        metrics_collector,
        dispatcher: dispatcher_data,
        pool: web::Data::new(pool),
    };

    let server = build_http_server(&bind, actix_workers, shutdown_timeout, deps)?;

    setup_graceful_shutdown(server.handle());

    info!("Server ready — listening on {}", bind);
    server.await?;

    Ok(())
}

/// Resolve bind address from env or use default.
fn resolve_bind_address() -> String {
    std::env::var("MARKETPLACE_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string())
}

/// Initialize the tracing/logging subscriber based on LOG_FORMAT env var.
fn init_tracing() {
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
}

/// Check whether the response cache is enabled via env var.
fn cache_enabled() -> bool {
    std::env::var("MARKETPLACE_CACHE_ENABLED")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
}

/// Parse cache size env vars and build Moka in-memory caches for listing & search responses.
fn build_moka_caches() -> (Cache<String, String>, Cache<String, String>, u64, u64) {
    let listing_cache_max_mb: u64 = std::env::var("LISTING_CACHE_MAX_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let search_cache_max_mb: u64 = std::env::var("SEARCH_CACHE_MAX_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let listing_cache_max_bytes = listing_cache_max_mb * 1024 * 1024;
    let search_cache_max_bytes = search_cache_max_mb * 1024 * 1024;

    let listing_cache: Cache<String, String> = Cache::builder()
        .weigher(|_key, value: &String| -> u32 { value.len() as u32 })
        .max_capacity(listing_cache_max_bytes)
        .time_to_live(std::time::Duration::from_secs(30 * 60))
        .build();
    let search_cache: Cache<String, String> = Cache::builder()
        .weigher(|_key, value: &String| -> u32 { value.len() as u32 })
        .max_capacity(search_cache_max_bytes)
        .time_to_live(std::time::Duration::from_secs(15 * 60))
        .build();

    info!(
        "Memory caches initialized: listing_cache={}MB, search_cache={}MB",
        listing_cache_max_mb, search_cache_max_mb
    );

    (listing_cache, search_cache, listing_cache_max_bytes, search_cache_max_bytes)
}

/// Create the broadcast channel for SSE-based negotiation event pushes.
fn init_event_bus() -> web::Data<broadcast::Sender<String>> {
    let (event_tx, _) = broadcast::channel::<String>(1024);
    web::Data::new(event_tx)
}

/// Set up the credit ledger system: repository, write-through cache, WAL recovery, and batch committer.
async fn init_ledger_system(
    pool: sqlx::postgres::PgPool,
    observability: Arc<ServerObservability>,
) -> Result<(web::Data<LedgerCache>, web::Data<crate::services::async_committer::BatchSender>), Box<dyn Error + Send + Sync>> {
    let ledger_repo: Arc<dyn CreditLedgerRepository> =
        Arc::new(PostgresCreditLedgerRepository::new(pool));
    let ledger_cache = LedgerCache::new(ledger_repo.clone(), Some(observability.clone()));
    let ledger_cache_data = web::Data::new(ledger_cache);

    info!("Recovering credit ledger WAL...");
    let wal = WalManager::from_env()
        .map_err(|e| std::io::Error::other(format!("WAL initialization failed: {e}")))?;
    wal.recover(&*ledger_repo)
        .await
        .map_err(|e| std::io::Error::other(format!("WAL recovery failed: {e}")))?;
    info!("WAL recovery complete");

    let (batch_tx, batch_committer) =
        batch_channel(ledger_repo, Arc::new(wal), Some(observability));
    batch_committer.start();
    let batch_tx_data = web::Data::new(batch_tx);

    Ok((ledger_cache_data, batch_tx_data))
}

/// Build the agent routing system: circuit breakers, registry, metrics, and HTTP dispatcher.
fn init_agent_system() -> AgentSystemDeps {
    let breaker_registry = web::Data::new(CircuitBreakerRegistry::default());
    let agent_registry = web::Data::new(AgentRegistry::default());
    let metrics_collector = web::Data::new(AgentMetricsCollector::default());

    let agent_dispatcher: Arc<dyn AgentDispatcher> =
        Arc::new(HttpAgentDispatcher::new(std::time::Duration::from_secs(30)));
    let dispatcher_data = web::Data::new(agent_dispatcher);

    (breaker_registry, agent_registry, metrics_collector, dispatcher_data)
}

/// Resolve actix worker count and shutdown timeout from env vars.
fn resolve_server_config() -> (usize, u64) {
    let num_cpus = num_cpus::get();
    let actix_workers = std::env::var("ACTIX_WORKERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| (num_cpus * 4).clamp(16, 64));
    let shutdown_timeout: u64 = std::env::var("SHUTDOWN_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    (actix_workers, shutdown_timeout)
}

/// Auto-run pending database migrations.
async fn run_migrations(pool: &sqlx::postgres::PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Running database schema migrations...");
    crate::bootstrap::apply_schema(pool)
        .await
        .map_err(|e| std::io::Error::other(format!("migration failed: {e}")))?;
    info!("Schema migrations complete");
    Ok(())
}

/// Shared application state registered as Actix `app_data` for all request handlers.
struct AppDependencies {
    app: web::Data<Arc<
        MarketplaceApp<
            PostgresListingRepository,
            PostgresIdempotencyKeyRepository,
            PostgresReservationLeaseRepository,
            PostgresContactRevealRepository,
        >,
    >>,
    observability: web::Data<Arc<ServerObservability>>,
    cache_enabled: web::Data<bool>,
    listing_cache: web::Data<Cache<String, String>>,
    search_cache: web::Data<Cache<String, String>>,
    listing_cache_limit: web::Data<u64>,
    search_cache_limit: web::Data<u64>,
    ledger_cache: web::Data<LedgerCache>,
    batch_tx: web::Data<crate::services::async_committer::BatchSender>,
    event_bus: web::Data<broadcast::Sender<String>>,
    breaker_registry: web::Data<CircuitBreakerRegistry>,
    agent_registry: web::Data<AgentRegistry>,
    metrics_collector: web::Data<AgentMetricsCollector>,
    dispatcher: web::Data<Arc<dyn AgentDispatcher>>,
    pool: web::Data<sqlx::postgres::PgPool>,
}

/// Build and return the Actix-web HTTP server, fully configured with middleware, app data, and routes.
fn build_http_server(
    bind: &str,
    actix_workers: usize,
    shutdown_timeout: u64,
    deps: AppDependencies,
) -> Result<actix_web::dev::Server, Box<dyn Error + Send + Sync>> {
    let obs_data = deps.observability.clone();

    info!("Starting Actix-web server on {}", bind);

    Ok(HttpServer::new(move || {
        let obs_for_mw = obs_data.clone();
        App::new()
            .wrap(actix_web::middleware::Compress::default())
            .wrap(TracingLogger::default())
            .wrap_fn(move |req, srv| {
                let obs = obs_for_mw.clone();
                let path = req.path().to_owned();
                let fut = srv.call(req);
                async move {
                    let res: Result<_, actix_web::Error> = fut.await;
                    if let Ok(ref response) = res {
                        obs.record_request(&path, response.status().as_u16());
                    }
                    res
                }
            })
            .app_data(deps.app.clone())
            .app_data(deps.observability.clone())
            .app_data(deps.cache_enabled.clone())
            .app_data(deps.listing_cache.clone())
            .app_data(deps.search_cache.clone())
            .app_data(deps.listing_cache_limit.clone())
            .app_data(deps.search_cache_limit.clone())
            .app_data(deps.ledger_cache.clone())
            .app_data(deps.batch_tx.clone())
            .app_data(deps.pool.clone())
            .app_data(deps.event_bus.clone())
            .app_data(deps.breaker_registry.clone())
            .app_data(deps.agent_registry.clone())
            .app_data(deps.metrics_collector.clone())
            .app_data(deps.dispatcher.clone())
            .route("/docs", web::get().to(crate::openapi::serve_swagger_editor))
            .route(
                "/api-docs/openapi.json",
                web::get().to(crate::openapi::serve_openapi_json),
            )
            .configure(crate::http::actix_handlers::register_api_routes)
            .route("/metrics", web::get().to(metrics_handler))
            .route("/health", web::get().to(health_check))
    })
    .bind(bind)?
    .workers(actix_workers)
    .shutdown_timeout(shutdown_timeout)
    .run())
}

/// Install a SIGINT/SIGTERM handler that triggers graceful server shutdown.
fn setup_graceful_shutdown(server_handle: actix_web::dev::ServerHandle) {
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutdown signal received, draining connections...");
        server_handle.stop(true).await;
        info!("Server stopped");
    });
}

/// Resolve the tokio worker thread count from env var or compute a sensible default.
/// Used both by `run()` to configure the runtime and by `metrics_handler()` to report.
fn resolve_worker_threads() -> usize {
    let num_cpus = num_cpus::get();
    let max_worker_threads = 8; // Cap at 8 for database-focused workload
    std::env::var("TOKIO_WORKER_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| num_cpus.saturating_sub(1).max(1).min(max_worker_threads))
}

async fn metrics_handler(
    pool: web::Data<sqlx::postgres::PgPool>,
    listing_cache: web::Data<Cache<String, String>>,
    search_cache: web::Data<Cache<String, String>>,
    listing_max_bytes: web::Data<u64>,
    search_max_bytes: web::Data<u64>,
    observability: web::Data<Arc<ServerObservability>>,
) -> impl actix_web::Responder {
    // Connection pool metrics
    let pool_size = pool.size();
    let idle_connections = pool.num_idle();

    // Runtime metrics
    let num_cpus = num_cpus::get();
    let max_worker_threads: usize = 8; // Cap at 8 for database-focused workload (must match resolve_worker_threads)
    let worker_threads = resolve_worker_threads();

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

    // Ledger observability snapshot (spec 0013 §4)
    let obs = observability.get_ref().snapshot();

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
         {http_counters}\
         # HELP ledger_cache_hit_total Total ledger cache hits\n# TYPE ledger_cache_hit_total counter\nledger_cache_hit_total {}\n\
         # HELP ledger_cache_miss_total Total ledger cache misses\n# TYPE ledger_cache_miss_total counter\nledger_cache_miss_total {}\n\
         # HELP ledger_batch_lag_milliseconds Duration from queue push to DB commit for the most recent batch\n# TYPE ledger_batch_lag_milliseconds gauge\nledger_batch_lag_milliseconds {}\n\
         # HELP ledger_batch_size Number of entries in the most recent flushed batch\n# TYPE ledger_batch_size gauge\nledger_batch_size {}\n",
         pool_size, idle_connections, connection_utilization, worker_threads, max_worker_threads, num_cpus,
         listing_count, listing_memory_mb, listing_max_mb, listing_cache_utilization,
         search_count, search_memory_mb, search_max_mb, search_cache_utilization,
         total_cache_mb,
         obs.ledger_cache_hit_total, obs.ledger_cache_miss_total,
         obs.ledger_batch_lag_milliseconds, obs.ledger_batch_size,
         http_counters = render_http_counter_metrics(&obs),
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
        .unwrap_or(50); // Safe default for small environments; increase to 200+ for high-concurrency benchmarks

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
