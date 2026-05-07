# Production Hardening Plan

> **Purpose**: Prepare the marketplace server for production deployment.
> **Scope**: Telemetry, structured logging, error handling, health checks, graceful shutdown.
> **Out of Scope**: CI/CD (planned for near future).

---

## Table of Contents

1. [Current State](#current-state)
2. [Telemetry & Observability](#1-telemetry--observability-high-priority)
3. [Structured Logging](#2-structured-logging-medium-priority)
4. [Error Handling Improvements](#3-error-handling-improvements-medium-priority)
5. [Health Checks (Deep)](#4-health-checks-deep-medium-priority)
6. [Graceful Shutdown](#5-graceful-shutdown-low-priority)
7. [Security Headers](#6-security-headers-low-priority)
8. [Implementation Order](#implementation-order)

---

## Current State

### What We Have ✅

- **Phase 1 Complete**: 7,281 ops/s (22.7× improvement)
- **Basic Actix server**: Running with Moka cache
- **Auth working**: `x-marketplace-claims` header
- **All tests pass**: 37 tests (35 unit + 2 integration)
- **Basic health endpoint**: `/health` returns `{"status": "ok"}`

### What We're Missing ⚠️

- **No structured logging**: Just `println!` and `eprintln!`
- **No metrics**: No visibility into request rates, latencies, cache hit rates
- **No tracing**: Can't trace requests across services
- **Basic error handling**: Generic error responses
- **No deep health checks**: Doesn't check DB connectivity
- **No graceful shutdown**: Kills connections abruptly

---

## 1. Telemetry & Observability (High Priority)

### Goal

Add tracing and metrics to understand:
- Request rates (requests/sec)
- Latency distribution (p50, p95, p99)
- Cache hit/miss rates (Moka)
- Error rates by endpoint
- Database query performance

### Dependencies

```toml
# backend/server/Cargo.toml
tracing = "0.1"
tracing-actix-web = "0.7"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
metrics = "0.22"
metrics-exporter-prometheus = "0.12"
```

### Implementation Sketch

#### 1.1: Add Tracing to Actix

```rust
// backend/server/src/http/actix_runtime.rs
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// In async_run():
tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new("info"))
    .with(tracing_subscriber::fmt::layer())
    .init();

// In HttpServer setup:
HttpServer::new(move || {
    App::new()
        .wrap(TracingLogger::default())  // Add tracing middleware
        .app_data(app_data.clone())
        // ... other routes
})
```

#### 1.2: Add Metrics (Prometheus)

```rust
// backend/server/src/observability.rs (or new file)
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics::{counter, histogram};

// Start Prometheus exporter (on /metrics endpoint)
let builder = PrometheusBuilder::new();
builder.install().expect("failed to install Prometheus exporter");

// In handlers, record metrics:
counter!("requests_total", "endpoint" => "/listings/search").increment(1);
histogram!("request_duration_seconds", "endpoint" => "/listings/search")
    .record(start.elapsed().as_secs_f64());
```

#### 1.3: Expose /metrics Endpoint

```rust
// Add to Actix routes:
.service(
    web::scope("/metrics")
        .route("", web::get().to(metrics_handler))
)
```

**Expected Impact**:
- ✅ Visibility into production performance
- ✅ Can create dashboards (Grafana)
- ✅ Alert on error rates, latency spikes
- ✅ Monitor cache hit rates (Moka metrics)

---

## 2. Structured Logging (Medium Priority)

### Goal

Replace `println!`/`eprintln!` with structured JSON logging.

### Dependencies

```toml
# Already have serde_json, just need:
tracing-serde = "0.1"
```

### Implementation

```rust
// Instead of:
eprintln!("CACHE HIT for {}", cache_key);

// Use:
tracing::info!(
    cache_key = %cache_key,
    event = "cache_hit",
    "Cache hit for search query"
);
```

**Benefits**:
- ✅ Logs are machine-parsable (JSON)
- ✅ Can filter by fields (cache_key, event type)
- ✅ Integrates with ELK stack, Datadog, etc.

---

## 3. Error Handling Improvements (Medium Priority)

### Goal

Return consistent, informative error responses with proper HTTP status codes.

### Current State

```rust
// In actix_handlers.rs
fn map_handler_error(e: &HandlerError) -> HttpResponse {
    match e {
        NotFound(_) => HttpResponse::NotFound(),
        // ... minimal mapping
    }
}
```

### Improvements

```rust
// Add error context:
#[derive(Serialize)]
struct ErrorResponse {
    error_code: String,
    message: String,
    request_id: String,  // For tracing
    timestamp: String,
}

// In handlers:
.catch(all).to(|err: actix_web::error::InternalError| {
    let response = ErrorResponse {
        error_code: "INTERNAL_ERROR".to_string(),
        message: err.to_string(),
        request_id: tracing::Span::current().id().map(|id| id.to_string()).unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    HttpResponse::InternalServerError().json(response)
})
```

**Benefits**:
- ✅ Consistent error format for clients
- ✅ Request ID for tracing errors
- ✅ Timestamps for debugging

---

## 4. Health Checks (Deep) (Medium Priority)

### Goal

Extend `/health` to check dependencies (DB, external services).

### Implementation

```rust
async fn health_check(
    pool: web::Data<sqlx::postgres::PgPool>,
) -> impl actix_web::Responder {
    let mut health = serde_json::json!({
        "status": "ok",
        "checks": {},
    });
    
    // Check database
    match sqlx::query("SELECT 1").execute(pool.get_ref()).await {
        Ok(_) => health["checks"]["database"] = serde_json::json!({"status": "ok"}),
        Err(e) => {
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
    
    (status, actix_web::HttpResponse::Ok().json(health))
}
```

**Benefits**:
- ✅ Kubernetes can use `/health` for liveness/readiness probes
- ✅ Alert on dependency failures
- ✅ Gradual degradation visibility

---

## 5. Graceful Shutdown (Low Priority)

### Goal

Allow in-flight requests to complete before shutting down.

### Actix Supports This Natively

```rust
HttpServer::new(move || { /* ... */ })
    .bind(&bind)?
    .run()
    .await?;
// Actix waits for workers to finish
```

**Just need to**:
- Listen for SIGINT/SIGTERM
- Close database connections gracefully
- Flush any buffers (Moka cache, logs)

---

## 6. Security Headers (Low Priority)

### Goal

Add security headers to all responses.

```rust
use actix_web::middleware::DefaultHeaders;

HttpServer::new(move || {
    App::new()
        .wrap(
            DefaultHeaders::new()
                .add(("X-Content-Type-Options", "nosniff"))
                .add(("X-Frame-Options", "DENY"))
                .add(("Content-Security-Policy", "default-src 'self'"))
        )
        // ... other middleware
})
```

---

## Implementation Order

### Phase A: Immediate (This Week)

1. ✅ Add `tracing` + `tracing-actix-web` for request logging
2. ✅ Add `metrics` + `metrics-exporter-prometheus` for /metrics endpoint
3. ✅ Implement deep health checks (DB connectivity)

### Phase B: Near Future (Next Sprint)

4. Structured logging throughout (replace println!)
5. Error handling improvements (consistent error format)
6. Graceful shutdown (if not already working)

### Phase C: Later (When Needed)

7. Security headers
8. Rate limiting (if abuse detected)
9. API versioning strategy

---

## Benchmarks to Maintain

After each change, run benchmark to ensure we don't regress:

```bash
cd backend && target/release/http_bench "http://127.0.0.1:3000" 5000
```

**Target**: Maintain **7,281 ops/s** (don't regress!)

---

## Skipped for Now 🚫

- **CI/CD**: Planned for near future (GitHub Actions, etc.)
- **Phase 2 (Zero-Copy)**: Skipped (low ROI, documented in `docs/future-plan.md`)
- **Phase 3 (Redis L2)**: Only if multi-instance deployment

---

**Document Status**: Living document (update as implemented)  
**Last Updated**: 2026-05-07  
**Author**: pi  
**Next Action**: Implement Phase A (tracing + metrics + deep health)
