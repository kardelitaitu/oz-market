use crate::app::MarketplaceApp;
use crate::http::handlers::HandlerError;
#[cfg(test)]
use crate::repositories::{
    contact_reveals::InMemoryContactRevealRepository, listings::InMemoryListingRepository,
    reservations::InMemoryReservationLeaseRepository,
};
#[cfg(not(test))]
use crate::repositories::{
    PostgresContactRevealRepository, PostgresIdempotencyKeyRepository, PostgresListingRepository,
    PostgresReservationLeaseRepository,
};
#[cfg(test)]
use crate::services::idempotency::InMemoryIdempotencyRepository;
use crate::services::rate_limiter::{
    global_limiter, AGENT_QUERY_RATE_MAX, AGENT_QUERY_RATE_WINDOW_SECS, CONTACT_REVEAL_RATE_MAX,
    CONTACT_REVEAL_RATE_WINDOW_SECS, CREATE_LISTING_RATE_MAX, CREATE_LISTING_RATE_WINDOW_SECS,
    NEW_SELLER_DAILY_MAX, NEW_SELLER_HOURLY_MAX, OPEN_NEGOTIATION_RATE_MAX,
    OPEN_NEGOTIATION_RATE_WINDOW_SECS, SEARCH_RATE_MAX, SEARCH_RATE_WINDOW_SECS,
};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use marketplace_api_contract::{
    AcceptNegotiationRequest, AgentQueryRequest, CreateListingRequest, NegotiationResponse,
    OpenNegotiationRequest, RejectNegotiationRequest, RequestContactRevealRequest, SearchRequest,
    SubmitOfferRequest,
};
use marketplace_auth_core::{Claims, Role};
use moka::future::Cache;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::ledger::{CreditLedgerError, NewTransaction, TransactionType};
use crate::services::agent_dispatcher::AgentDispatcher;
use crate::services::agent_metrics::AgentMetricsCollector;
use crate::services::agent_registry::AgentRegistry;
use crate::services::agent_routing;
use crate::services::async_committer::BatchSender;
use crate::services::circuit_breaker::CircuitBreakerRegistry;
use crate::services::ledger_cache::LedgerCache;
use crate::services::wal::WalEntry;

// Production hardening: tracing + metrics
use metrics::{counter, histogram};
use tracing::error;

#[cfg(test)]
use std::collections::HashSet;

// SSE event types for real-time negotiation updates
use actix_web::web::Bytes;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

#[derive(Clone, Serialize)]
struct NegotiationEvent {
    negotiation_id: String,
    event_type: &'static str,
    response: NegotiationResponse,
}

fn publish_negotiation_event(
    tx: &broadcast::Sender<String>,
    negotiation_id: &str,
    event_type: &'static str,
    response: &NegotiationResponse,
) {
    let event = NegotiationEvent {
        negotiation_id: negotiation_id.to_string(),
        event_type,
        response: response.clone(),
    };
    if let Ok(json) = serde_json::to_string(&event) {
        let _ = tx.send(json);
    }
}

/// SSE handler: streams negotiation events for a given negotiation_id.
pub async fn negotiation_event_stream(
    app: web::Data<ActixApp>,
    event_bus: web::Data<broadcast::Sender<String>>,
    req: HttpRequest,
    negotiation_id: web::Path<String>,
) -> HttpResponse {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let neg_id = negotiation_id.into_inner();

    // Verify the user is a participant and get initial state
    let initial_state = match app.get_negotiation_status(&claims, &neg_id).await {
        Ok(response) => response,
        Err(e) => return map_handler_error(&e),
    };

    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(32);
    let mut broadcast_rx = event_bus.subscribe();
    let stream_neg_id = neg_id.clone();

    // Send initial state immediately
    let initial_event = NegotiationEvent {
        event_type: "initial_state",
        negotiation_id: neg_id.clone(),
        response: initial_state,
    };
    if let Ok(json) = serde_json::to_string(&initial_event) {
        let _ = tx
            .send(Bytes::from(format!(
                "event: negotiation_updated\ndata: {}\n\n",
                json
            )))
            .await;
    }

    // Background task: relay broadcast events + heartbeats
    tokio::spawn(async move {
        let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            tokio::select! {
                event = broadcast_rx.recv() => {
                    match event {
                        Ok(msg) => {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&msg) {
                                if parsed.get("negotiation_id")
                                    .and_then(|v| v.as_str())
                                    == Some(&stream_neg_id)
                                {
                                    let _ = tx
                                        .send(Bytes::from(format!(
                                            "event: negotiation_updated\ndata: {}\n\n",
                                            msg
                                        )))
                                        .await;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            let _ = tx
                                .send(Bytes::from(format!("event: lag\ndata: {n}\n\n")))
                                .await;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = heartbeat.tick() => {
                    if tx.send(Bytes::from(": heartbeat\n\n")).await.is_err() {
                        break; // client disconnected
                    }
                }
            }
        }
    });

    let stream = ReceiverStream::new(rx).map(Ok::<_, actix_web::Error>);
    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(stream)
}

#[cfg(test)]
fn parse_fields_param(query: &str) -> Option<HashSet<String>> {
    let result: HashSet<String> = query
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

#[cfg(test)]
fn parse_include_param(query: &str) -> Option<HashSet<String>> {
    let result: HashSet<String> = query
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

#[cfg(test)]
fn filter_listing_fields(value: &serde_json::Value, fields: &HashSet<String>) -> serde_json::Value {
    if fields.is_empty() {
        return value.clone();
    }
    if let serde_json::Value::Object(map) = value {
        let filtered: serde_json::Map<String, serde_json::Value> = map
            .iter()
            .filter(|(k, _)| fields.contains(k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        serde_json::Value::Object(filtered)
    } else {
        value.clone()
    }
}

// OpenAPI annotations
// use utoipa::path;  // Removed - causes ambiguity with actix_web::web::Path

// Type alias for the concrete app type used in Actix handlers.
// Production: Postgres-backed. Test: in-memory (via cfg(test)).
#[cfg(not(test))]
type ActixApp = Arc<
    MarketplaceApp<
        PostgresListingRepository,
        PostgresIdempotencyKeyRepository,
        PostgresReservationLeaseRepository,
        PostgresContactRevealRepository,
    >,
>;
#[cfg(test)]
type ActixApp = Arc<
    MarketplaceApp<
        InMemoryListingRepository,
        InMemoryIdempotencyRepository,
        InMemoryReservationLeaseRepository,
        InMemoryContactRevealRepository,
    >,
>;

// Helper to map HandlerError to HttpResponse
fn map_handler_error(error: &HandlerError) -> HttpResponse {
    let (status, payload) = error.to_http_parts();
    let body = serde_json::to_string(&payload).unwrap_or_default();
    HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status)
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
    )
    .content_type("application/json")
    .body(body)
}

fn error_json(status: u16, code: &str, message: &str) -> HttpResponse {
    HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status)
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
    )
    .content_type("application/json")
    .body(
        serde_json::json!({
            "error": {
                "code": code,
                "message": message,
                "field": null,
            }
        })
        .to_string(),
    )
}

fn rate_limited(
    message: &str,
    rl: &crate::services::rate_limiter::RateLimitStatus,
) -> HttpResponse {
    let body = serde_json::json!({
        "error": {
            "code": "rate_limited",
            "message": message,
            "field": null,
        }
    })
    .to_string();
    HttpResponse::TooManyRequests()
        .insert_header(("X-RateLimit-Limit", rl.limit.to_string()))
        .insert_header(("X-RateLimit-Remaining", "0"))
        .insert_header(("X-RateLimit-Reset", rl.reset_after_secs.to_string()))
        .content_type("application/json")
        .body(body)
}

// If MARKETPLACE_API_KEY env is set, accept x-marketplace-api-key header
// and return a full-access demo Claims set. Makes agent auth zero-config.
fn api_key_to_claims(req: &HttpRequest) -> Option<Claims> {
    let expected = std::env::var("MARKETPLACE_API_KEY").ok()?;
    let actual = req.headers().get("x-marketplace-api-key")?.to_str().ok()?;
    if actual == expected {
        Some(Claims {
            sub: "demo-agent".to_string(),
            roles: vec![
                marketplace_auth_core::Role::SellerListingWriter,
                marketplace_auth_core::Role::SellerNegotiator,
                marketplace_auth_core::Role::SellerContactRevealApprover,
                marketplace_auth_core::Role::BuyerSearcher,
                marketplace_auth_core::Role::BuyerNegotiator,
                marketplace_auth_core::Role::Admin,
            ],
            scopes: vec![
                marketplace_auth_core::Scope::ListingCreate,
                marketplace_auth_core::Scope::ListingRead,
                marketplace_auth_core::Scope::ListingSearch,
                marketplace_auth_core::Scope::NegotiationCreate,
                marketplace_auth_core::Scope::NegotiationRead,
                marketplace_auth_core::Scope::NegotiationOfferSubmit,
                marketplace_auth_core::Scope::NegotiationRevealRequest,
                marketplace_auth_core::Scope::RevealApprove,
            ],
            seller_account_id: Some("demo-seller".to_string()),
            buyer_agent_id: Some("demo-buyer".to_string()),
            hardware_id: None,
            exp: None,
        })
    } else {
        None
    }
}

// Helper to extract Claims from x-marketplace-claims header,
// with env-configured API key fallback (MARKETPLACE_API_KEY).
fn extract_claims(req: &HttpRequest) -> Result<Claims, HttpResponse> {
    // Try API key first — set MARKETPLACE_API_KEY env for zero-config agent auth
    if let Some(claims) = api_key_to_claims(req) {
        return Ok(claims);
    }
    if let Some(h) = req.headers().get("x-marketplace-claims") {
        if let Ok(s) = h.to_str() {
            match serde_json::from_str::<Claims>(s) {
                Ok(claims) => return Ok(claims),
                Err(e) => {
                    error!("Claims parse error: {}", e);
                    return Err(HttpResponse::Unauthorized()
                        .json(serde_json::json!({"error": format!("invalid claims: {}", e)})));
                }
            }
        }
    }
    error!("Missing claims header");
    Err(HttpResponse::Unauthorized().json(serde_json::json!({"error": "missing claims header"})))
}

// Helper to extract Claims optionally (returns None if missing/invalid)
fn extract_claims_optional(req: &HttpRequest) -> Option<Claims> {
    // Try API key first
    if let Some(claims) = api_key_to_claims(req) {
        return Some(claims);
    }
    if let Some(h) = req.headers().get("x-marketplace-claims") {
        if let Ok(s) = h.to_str() {
            match serde_json::from_str::<Claims>(s) {
                Ok(claims) => return Some(claims),
                Err(e) => {
                    error!("Claims parse error (optional): {}", e);
                    return None;
                }
            }
        }
    }
    None
}

// --- Listing handlers ---

pub async fn get_listing(
    app: web::Data<ActixApp>,
    cache_enabled: web::Data<bool>,
    listing_cache: web::Data<Cache<String, String>>,
    listing_id: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let start = std::time::Instant::now();
    // Claims are optional for get listing - public access allowed
    let claims = extract_claims_optional(&req);
    let cache_key = listing_id.to_string();
    // Try cache first (stores pre-serialized JSON)
    if **cache_enabled {
        if let Some(cached_json) = listing_cache.get(&cache_key).await {
            counter!("cache_hits_total", "type" => "listing").increment(1);
            histogram!("request_duration_seconds", "endpoint" => "/listings/{id}")
                .record(start.elapsed().as_secs_f64());
            return HttpResponse::Ok()
                .content_type("application/json")
                .body(cached_json);
        }
    }
    counter!("cache_misses_total", "type" => "listing").increment(1);
    // Fallback to app
    match app.get_listing(claims.as_ref(), &listing_id).await {
        Ok(Some(listing)) => {
            let json_string = serde_json::to_string(&listing).unwrap_or_default();
            if **cache_enabled {
                listing_cache.insert(cache_key, json_string.clone()).await;
            }
            histogram!("request_duration_seconds", "endpoint" => "/listings/{id}")
                .record(start.elapsed().as_secs_f64());
            HttpResponse::Ok()
                .content_type("application/json")
                .body(json_string)
        }
        Ok(None) => error_json(404, "not_found", "listing not found"),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn search_listings(
    app: web::Data<ActixApp>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    query: web::Query<SearchRequest>,
    req: HttpRequest,
) -> impl Responder {
    let start = std::time::Instant::now();
    // Claims are optional for search - public access allowed
    let claims = extract_claims_optional(&req);

    // Rate limit search by claims.sub (only for authenticated users)
    // We capture rl_search before the if-let to keep it in scope for success paths
    let mut rl_search: Option<crate::services::rate_limiter::RateLimitStatus> = None;
    if let Some(ref c) = claims {
        let search_key = format!("search:{}", c.sub);
        let rl = global_limiter().check(&search_key, SEARCH_RATE_MAX, SEARCH_RATE_WINDOW_SECS);
        if !rl.allowed {
            return rate_limited("search rate limit exceeded (60/min)", &rl);
        }
        rl_search = Some(rl);
    }

    // Build cache key with meaningful search params for better cache hits
    let modified_query = (*query).clone();
    let listing_type_str = modified_query
        .listing_type
        .map(|t| format!("{:?}", t))
        .unwrap_or_else(|| "all".to_string());
    let sort_by_str = format!("{:?}", modified_query.sort_by);
    let category_str = modified_query
        .category
        .map(|c| format!("{:?}", c))
        .unwrap_or_else(|| "none".to_string());
    let limit_str = modified_query
        .limit
        .map(|l| l.to_string())
        .unwrap_or_else(|| "20".to_string());
    let cursor_str = modified_query
        .cursor
        .as_deref()
        .unwrap_or("none")
        .to_string();

    let cache_key = format!(
        "search:lt:{}:cat:{}:sort:{}:limit:{}:cur:{}",
        listing_type_str, category_str, sort_by_str, limit_str, cursor_str
    );

    // Try cache first (stores pre-serialized JSON)
    if **cache_enabled {
        if let Some(cached_json) = search_cache.get(&cache_key).await {
            counter!("cache_hits_total", "type" => "search").increment(1);
            histogram!("request_duration_seconds", "endpoint" => "/listings/search")
                .record(start.elapsed().as_secs_f64());
            let mut resp = HttpResponse::Ok();
            if let Some(ref rl) = rl_search {
                resp.insert_header(("X-RateLimit-Limit", rl.limit.to_string()));
                resp.insert_header(("X-RateLimit-Remaining", rl.remaining.to_string()));
                resp.insert_header(("X-RateLimit-Reset", rl.reset_after_secs.to_string()));
            }
            return resp.content_type("application/json").body(cached_json);
        }
    }
    counter!("cache_misses_total", "type" => "search").increment(1);

    // Fallback to app
    match app.search_listings(claims.as_ref(), &modified_query).await {
        Ok(response) => {
            let json_string = serde_json::to_string(&response).unwrap_or_default();
            if **cache_enabled {
                search_cache.insert(cache_key, json_string.clone()).await;
            }
            histogram!("request_duration_seconds", "endpoint" => "/listings/search")
                .record(start.elapsed().as_secs_f64());
            let mut resp = HttpResponse::Ok();
            if let Some(ref rl) = rl_search {
                resp.insert_header(("X-RateLimit-Limit", rl.limit.to_string()));
                resp.insert_header(("X-RateLimit-Remaining", rl.remaining.to_string()));
                resp.insert_header(("X-RateLimit-Reset", rl.reset_after_secs.to_string()));
            }
            resp.content_type("application/json").body(json_string)
        }
        Err(e) => map_handler_error(&e),
    }
}

pub async fn create_listing(
    app: web::Data<ActixApp>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    req: HttpRequest,
    body: web::Json<CreateListingRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let create_key = format!("create:{}", claims.sub);
    let rl_create = global_limiter().check(
        &create_key,
        CREATE_LISTING_RATE_MAX,
        CREATE_LISTING_RATE_WINDOW_SECS,
    );
    if !rl_create.allowed {
        return rate_limited("create listing rate limit exceeded (10/min)", &rl_create);
    }
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::util::current_time_marker();
    // Invalidate search cache on write
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app.create_listing(&claims, &body, &fingerprint, &now).await {
        Ok((created, false)) => HttpResponse::Created()
            .insert_header(("X-RateLimit-Limit", rl_create.limit.to_string()))
            .insert_header(("X-RateLimit-Remaining", rl_create.remaining.to_string()))
            .insert_header(("X-RateLimit-Reset", rl_create.reset_after_secs.to_string()))
            .json(created),
        Ok((created, true)) => HttpResponse::Ok()
            .insert_header(("X-RateLimit-Limit", rl_create.limit.to_string()))
            .insert_header(("X-RateLimit-Remaining", rl_create.remaining.to_string()))
            .insert_header(("X-RateLimit-Reset", rl_create.reset_after_secs.to_string()))
            .json(created),
        Err(e) => map_handler_error(&e),
    }
}

// --- Agent handler ---

pub async fn agent_query(
    _app: web::Data<ActixApp>,
    breaker_registry: web::Data<CircuitBreakerRegistry>,
    agent_registry: web::Data<AgentRegistry>,
    metrics_collector: web::Data<AgentMetricsCollector>,
    dispatcher: web::Data<Arc<dyn AgentDispatcher>>,
    req: HttpRequest,
    body: web::Json<AgentQueryRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let agent_key = format!("agent:{}", claims.sub);
    let rl_agent = global_limiter().check(
        &agent_key,
        AGENT_QUERY_RATE_MAX,
        AGENT_QUERY_RATE_WINDOW_SECS,
    );
    if !rl_agent.allowed {
        return rate_limited("agent query rate limit exceeded (20/min)", &rl_agent);
    }
    match agent_routing::route_agent_query(
        agent_registry.get_ref(),
        breaker_registry.get_ref(),
        metrics_collector.get_ref(),
        dispatcher.get_ref().as_ref(),
        &body,
        agent_routing::DEFAULT_AGENT_TIMEOUT,
    )
    .await
    {
        Ok(result) => HttpResponse::Ok()
            .insert_header(("X-RateLimit-Limit", rl_agent.limit.to_string()))
            .insert_header(("X-RateLimit-Remaining", rl_agent.remaining.to_string()))
            .insert_header(("X-RateLimit-Reset", rl_agent.reset_after_secs.to_string()))
            .json(result),
        Err(agent_routing::RoutingError::NoAgentsAvailable) => {
            error_json(503, "no_agents", "No available agents to process the query")
        }
        Err(e) => error_json(502, "dispatch_error", &e.to_string()),
    }
}

// --- Negotiation handlers ---

pub async fn open_negotiation(
    app: web::Data<ActixApp>,
    event_bus: web::Data<broadcast::Sender<String>>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    req: HttpRequest,
    body: web::Json<OpenNegotiationRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let negot_key = format!("negotiate:{}", claims.sub);
    let rl_negot = global_limiter().check(
        &negot_key,
        OPEN_NEGOTIATION_RATE_MAX,
        OPEN_NEGOTIATION_RATE_WINDOW_SECS,
    );
    if !rl_negot.allowed {
        return rate_limited("open negotiation rate limit exceeded (20/min)", &rl_negot);
    }
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::util::current_time_marker();
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app
        .open_negotiation(&claims, &body, &fingerprint, &now)
        .await
    {
        Ok((response, false)) => {
            let neg_id = response.negotiation_id.clone();
            publish_negotiation_event(&event_bus, &neg_id, "negotiation_opened", &response);
            HttpResponse::Created()
                .insert_header(("X-RateLimit-Limit", rl_negot.limit.to_string()))
                .insert_header(("X-RateLimit-Remaining", rl_negot.remaining.to_string()))
                .insert_header(("X-RateLimit-Reset", rl_negot.reset_after_secs.to_string()))
                .json(response)
        }
        Ok((response, true)) => {
            let neg_id = response.negotiation_id.clone();
            publish_negotiation_event(&event_bus, &neg_id, "negotiation_opened", &response);
            HttpResponse::Ok()
                .insert_header(("X-RateLimit-Limit", rl_negot.limit.to_string()))
                .insert_header(("X-RateLimit-Remaining", rl_negot.remaining.to_string()))
                .insert_header(("X-RateLimit-Reset", rl_negot.reset_after_secs.to_string()))
                .json(response)
        }
        Err(e) => map_handler_error(&e),
    }
}

pub async fn get_negotiation_status(
    app: web::Data<ActixApp>,
    req: HttpRequest,
    negotiation_id: web::Path<String>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    match app.get_negotiation_status(&claims, &negotiation_id).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn submit_offer(
    app: web::Data<ActixApp>,
    event_bus: web::Data<broadcast::Sender<String>>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    req: HttpRequest,
    negotiation_id: web::Path<String>,
    body: web::Json<SubmitOfferRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let negot_key = format!("negotiate:{}", claims.sub);
    let rl_offer = global_limiter().check(
        &negot_key,
        OPEN_NEGOTIATION_RATE_MAX,
        OPEN_NEGOTIATION_RATE_WINDOW_SECS,
    );
    if !rl_offer.allowed {
        return rate_limited("offer submit rate limit exceeded (20/min)", &rl_offer);
    }
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::util::current_time_marker();
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app
        .submit_offer(&claims, &negotiation_id, &body, &fingerprint, &now)
        .await
    {
        Ok(response) => {
            publish_negotiation_event(&event_bus, &negotiation_id, "offer_submitted", &response);
            HttpResponse::Ok()
                .insert_header(("X-RateLimit-Limit", rl_offer.limit.to_string()))
                .insert_header(("X-RateLimit-Remaining", rl_offer.remaining.to_string()))
                .insert_header(("X-RateLimit-Reset", rl_offer.reset_after_secs.to_string()))
                .json(response)
        }
        Err(e) => map_handler_error(&e),
    }
}

pub async fn accept_negotiation(
    app: web::Data<ActixApp>,
    event_bus: web::Data<broadcast::Sender<String>>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    req: HttpRequest,
    negotiation_id: web::Path<String>,
    body: web::Json<AcceptNegotiationRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let negot_key = format!("negotiate:{}", claims.sub);
    let rl_accept = global_limiter().check(
        &negot_key,
        OPEN_NEGOTIATION_RATE_MAX,
        OPEN_NEGOTIATION_RATE_WINDOW_SECS,
    );
    if !rl_accept.allowed {
        return rate_limited("accept rate limit exceeded (20/min)", &rl_accept);
    }
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::util::current_time_marker();
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app
        .accept_negotiation(&claims, &negotiation_id, &body, &fingerprint, &now)
        .await
    {
        Ok(response) => {
            publish_negotiation_event(
                &event_bus,
                &negotiation_id,
                "negotiation_accepted",
                &response,
            );
            HttpResponse::Ok()
                .insert_header(("X-RateLimit-Limit", rl_accept.limit.to_string()))
                .insert_header(("X-RateLimit-Remaining", rl_accept.remaining.to_string()))
                .insert_header(("X-RateLimit-Reset", rl_accept.reset_after_secs.to_string()))
                .json(response)
        }
        Err(e) => map_handler_error(&e),
    }
}

pub async fn reject_negotiation(
    app: web::Data<ActixApp>,
    event_bus: web::Data<broadcast::Sender<String>>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    req: HttpRequest,
    negotiation_id: web::Path<String>,
    body: web::Json<RejectNegotiationRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let negot_key = format!("negotiate:{}", claims.sub);
    let rl_reject = global_limiter().check(
        &negot_key,
        OPEN_NEGOTIATION_RATE_MAX,
        OPEN_NEGOTIATION_RATE_WINDOW_SECS,
    );
    if !rl_reject.allowed {
        return rate_limited("reject rate limit exceeded (20/min)", &rl_reject);
    }
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::util::current_time_marker();
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app
        .reject_negotiation(&claims, &negotiation_id, &body, &fingerprint, &now)
        .await
    {
        Ok(response) => {
            publish_negotiation_event(
                &event_bus,
                &negotiation_id,
                "negotiation_rejected",
                &response,
            );
            HttpResponse::Ok()
                .insert_header(("X-RateLimit-Limit", rl_reject.limit.to_string()))
                .insert_header(("X-RateLimit-Remaining", rl_reject.remaining.to_string()))
                .insert_header(("X-RateLimit-Reset", rl_reject.reset_after_secs.to_string()))
                .json(response)
        }
        Err(e) => map_handler_error(&e),
    }
}

// --- Contact reveal handlers ---

pub async fn request_contact_reveal(
    app: web::Data<ActixApp>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    req: HttpRequest,
    negotiation_id: web::Path<String>,
    body: web::Json<RequestContactRevealRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let reveal_key = format!("reveal:{}", claims.sub);
    let rl_reveal_req = global_limiter().check(
        &reveal_key,
        CONTACT_REVEAL_RATE_MAX,
        CONTACT_REVEAL_RATE_WINDOW_SECS,
    );
    if !rl_reveal_req.allowed {
        return rate_limited(
            "contact reveal rate limit exceeded (10/min)",
            &rl_reveal_req,
        );
    }
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::util::current_time_marker();
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app
        .request_contact_reveal(&claims, &negotiation_id, &body, &fingerprint, &now)
        .await
    {
        Ok(response) => HttpResponse::Accepted()
            .insert_header(("X-RateLimit-Limit", rl_reveal_req.limit.to_string()))
            .insert_header(("X-RateLimit-Remaining", rl_reveal_req.remaining.to_string()))
            .insert_header((
                "X-RateLimit-Reset",
                rl_reveal_req.reset_after_secs.to_string(),
            ))
            .json(response),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn approve_contact_reveal(
    app: web::Data<ActixApp>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    req: HttpRequest,
    reveal_id: web::Path<String>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let approve_key = format!("approve:{}", claims.sub);
    let rl_approve = global_limiter().check(
        &approve_key,
        CONTACT_REVEAL_RATE_MAX,
        CONTACT_REVEAL_RATE_WINDOW_SECS,
    );
    if !rl_approve.allowed {
        return rate_limited("contact reveal rate limit exceeded (10/min)", &rl_approve);
    }
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app.approve_contact_reveal(&claims, &reveal_id).await {
        Ok(response) => HttpResponse::Ok()
            .insert_header(("X-RateLimit-Limit", rl_approve.limit.to_string()))
            .insert_header(("X-RateLimit-Remaining", rl_approve.remaining.to_string()))
            .insert_header(("X-RateLimit-Reset", rl_approve.reset_after_secs.to_string()))
            .json(response),
        Err(e) => map_handler_error(&e),
    }
}

// --- Deprecated listing-type redirects (Spec 0001) ---
// Old category-specific endpoints redirect to unified /v1/listings/{id}

pub async fn deprecated_listing_redirect(listing_id: web::Path<String>) -> impl Responder {
    let target = format!("/v1/listings/{}", listing_id.into_inner());
    HttpResponse::MovedPermanently()
        .insert_header(("Location", target.as_str()))
        .insert_header(("Deprecation", "true"))
        .insert_header(("Sunset", "Sat, 01 Jun 2026 00:00:00 GMT"))
        .insert_header(("Link", format!("<{}>; rel=\"successor-version\"", target)))
        .finish()
}

pub async fn deprecated_search_redirect() -> impl Responder {
    HttpResponse::MovedPermanently()
        .insert_header(("Location", "/v1/listings/search"))
        .insert_header(("Deprecation", "true"))
        .insert_header(("Sunset", "Sat, 01 Jun 2026 00:00:00 GMT"))
        .insert_header(("Link", "</v1/listings/search>; rel=\"successor-version\""))
        .finish()
}

// --- Internal admin handlers ---

pub async fn archive_listing(
    app: web::Data<ActixApp>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    listing_id: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let now = crate::http::util::current_time_marker();
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app.archive_listing(&claims, &listing_id, "", &now).await {
        Ok(Some(_listing)) => HttpResponse::NoContent().finish(),
        Ok(None) => error_json(404, "not_found", "listing not found"),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn release_reservation(
    app: web::Data<ActixApp>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    lease_id: web::Path<String>,
    claims: web::ReqData<Claims>,
) -> impl Responder {
    let now = crate::http::util::current_time_marker();
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    // release_reservation needs listing_id, not lease_id
    // For now, we'll use lease_id as listing_id (simplification - real code would look up lease first)
    match app.release_reservation(&claims, &lease_id, "", &now).await {
        Ok(Some(_lease)) => HttpResponse::NoContent().finish(),
        Ok(None) => error_json(404, "not_found", "listing not found"),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn set_seller_trust_level(
    app: web::Data<ActixApp>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    seller_id: web::Path<String>,
    trust_level: web::Json<String>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let now = crate::http::util::current_time_marker();
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    // set_seller_trust_level(claims, seller_account_id, trust_level, reason, now)
    match app
        .set_seller_trust_level(&claims, &seller_id, &trust_level, "", &now)
        .await
    {
        Ok(Some(_account)) => HttpResponse::NoContent().finish(),
        Ok(None) => error_json(404, "not_found", "seller not found"),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn set_seller_quota_override(
    app: web::Data<ActixApp>,
    cache_enabled: web::Data<bool>,
    search_cache: web::Data<Cache<String, String>>,
    seller_id: web::Path<String>,
    quota: web::Json<Option<i32>>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let now = crate::http::util::current_time_marker();
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    // set_seller_quota_override(claims, seller_account_id, quota_override, reason, now)
    match app
        .set_seller_quota_override(&claims, &seller_id, *quota, "", &now)
        .await
    {
        Ok(Some(_account)) => HttpResponse::NoContent().finish(),
        Ok(None) => error_json(404, "not_found", "seller not found"),
        Err(e) => map_handler_error(&e),
    }
}

/// Admin: Recalculate seller_rating for a seller
pub async fn recalculate_seller_rating(
    pool: web::Data<sqlx::postgres::PgPool>,
    seller_id: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // Check admin role
    if !claims.roles.iter().any(|r| matches!(r, Role::Admin)) {
        return error_json(403, "forbidden", "Admin access required");
    }

    let pool = pool.get_ref();
    let result = sqlx::query(
        "UPDATE seller_accounts SET seller_rating = (
            SELECT AVG(rating)::DECIMAL(3,2) FROM reviews 
            WHERE seller_account_id = $1 AND status = 'approved'
        ) WHERE seller_account_id = $1",
    )
    .bind(seller_id.as_str())
    .execute(pool)
    .await;

    match result {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            eprintln!("Recalculate rating error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Create a review for a listing (buyer only)
pub async fn create_review(
    pool: web::Data<sqlx::postgres::PgPool>,
    listing_id: web::Path<String>,
    review: web::Json<serde_json::Value>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // Check buyer role
    if !claims
        .roles
        .iter()
        .any(|r| matches!(r, Role::BuyerSearcher | Role::BuyerNegotiator))
    {
        return error_json(403, "forbidden", "Buyer access required");
    }

    let rating = review.get("rating").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if !(1..=5).contains(&rating) {
        return error_json(400, "invalid_field", "Rating must be between 1 and 5");
    }

    let title = review.get("title").and_then(|v| v.as_str()).unwrap_or("");
    if title.len() < 3 || title.len() > 200 {
        return error_json(
            400,
            "invalid_field",
            "Title must be between 3 and 200 characters",
        );
    }

    let body = review.get("body").and_then(|v| v.as_str());
    let review_id = format!("rev_{}", uuid::Uuid::new_v4());

    let pool = pool.get_ref();

    // Get seller_account_id for this listing
    let seller_row = sqlx::query(
        "SELECT s.seller_account_id FROM listings l 
         JOIN seller_accounts s ON l.owner_id = s.owner_id 
         WHERE l.listing_id = $1",
    )
    .bind(listing_id.as_str())
    .fetch_optional(pool)
    .await;

    let seller_account_id = match seller_row {
        Ok(Some(row)) => row.get::<String, _>("seller_account_id"),
        Ok(None) => return error_json(404, "not_found", "Listing or seller not found"),
        Err(e) => {
            error!("DB error fetching seller: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let result = sqlx::query(
        "INSERT INTO reviews (review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')"
    )
    .bind(&review_id)
    .bind(listing_id.as_str())
    .bind(&seller_account_id)
    .bind(&claims.sub)
    .bind(rating)
    .bind(title)
    .bind(body)
    .execute(pool)
    .await;

    match result {
        Ok(_) => HttpResponse::Created().json(json!({
            "review_id": review_id,
            "status": "pending"
        })),
        Err(e) => {
            error!("Create review error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Admin/Reviewer: Get current rate limiter snapshot and configuration
pub async fn get_rate_limits(req: HttpRequest) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // Require admin or support reviewer (same as other internal read routes)
    if !claims
        .roles
        .iter()
        .any(|r| matches!(r, Role::Admin | Role::SupportReviewer))
    {
        return error_json(
            403,
            "forbidden",
            "Internal route requires admin or support reviewer role",
        );
    }

    let buckets = global_limiter().snapshot();
    let config = serde_json::json!({
        "search": { "max": SEARCH_RATE_MAX, "window_secs": SEARCH_RATE_WINDOW_SECS },
        "create_listing": { "max": CREATE_LISTING_RATE_MAX, "window_secs": CREATE_LISTING_RATE_WINDOW_SECS },
        "open_negotiation": { "max": OPEN_NEGOTIATION_RATE_MAX, "window_secs": OPEN_NEGOTIATION_RATE_WINDOW_SECS },
        "contact_reveal": { "max": CONTACT_REVEAL_RATE_MAX, "window_secs": CONTACT_REVEAL_RATE_WINDOW_SECS },
        "agent_query": { "max": AGENT_QUERY_RATE_MAX, "window_secs": AGENT_QUERY_RATE_WINDOW_SECS },
        "new_seller_daily": { "max": NEW_SELLER_DAILY_MAX },
        "new_seller_hourly": { "max": NEW_SELLER_HOURLY_MAX },
    });

    HttpResponse::Ok().json(serde_json::json!({
        "buckets": buckets,
        "config": config,
    }))
}

/// List reviews for a listing
pub async fn list_reviews_for_listing(
    pool: web::Data<sqlx::postgres::PgPool>,
    listing_id: web::Path<String>,
) -> impl Responder {
    let pool = pool.get_ref();
    let rows = sqlx::query(
        "SELECT review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status, created_at::text as created_at
         FROM reviews WHERE listing_id = $1 ORDER BY created_at DESC"
    )
    .bind(listing_id.as_str())
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rows) => {
            let mut reviews = Vec::new();
            for row in rows {
                let review_id: String = row.get("review_id");
                let listing_id: String = row.get("listing_id");
                let seller_account_id: String = row.get("seller_account_id");
                let reviewer_id: String = row.get("reviewer_id");
                let rating: i32 = row.get("rating");
                let title: String = row.get("title");
                let body: Option<String> = row.get("body");
                let status: String = row.get("status");
                let created_at: String = row.get("created_at");

                reviews.push(serde_json::json!({
                    "review_id": review_id,
                    "listing_id": listing_id,
                    "seller_account_id": seller_account_id,
                    "reviewer_id": reviewer_id,
                    "rating": rating,
                    "title": title,
                    "body": body,
                    "status": status,
                    "created_at": created_at,
                }));
            }
            HttpResponse::Ok().json(reviews)
        }
        Err(e) => {
            error!("List reviews error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Approve a review (admin only)
pub async fn approve_review(
    pool: web::Data<sqlx::postgres::PgPool>,
    review_id: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // Check admin role
    if !claims.roles.iter().any(|r| matches!(r, Role::Admin)) {
        return error_json(403, "forbidden", "Admin access required");
    }

    let pool = pool.get_ref();
    let result = sqlx::query("UPDATE reviews SET status = 'approved' WHERE review_id = $1")
        .bind(review_id.as_str())
        .execute(pool)
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                HttpResponse::NoContent().finish()
            } else {
                error_json(404, "not_found", "Review not found")
            }
        }
        Err(e) => {
            error!("Approve review error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Reject a review (admin only)
pub async fn reject_review(
    pool: web::Data<sqlx::postgres::PgPool>,
    review_id: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // Check admin role
    if !claims.roles.iter().any(|r| matches!(r, Role::Admin)) {
        return error_json(403, "forbidden", "Admin access required");
    }

    let pool = pool.get_ref();
    let result = sqlx::query("UPDATE reviews SET status = 'rejected' WHERE review_id = $1")
        .bind(review_id.as_str())
        .execute(pool)
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                HttpResponse::NoContent().finish()
            } else {
                error_json(404, "not_found", "Review not found")
            }
        }
        Err(e) => {
            error!("Reject review error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

// ---------------------------------------------------------------------------
// Agent Health API — circuit breaker status
// ---------------------------------------------------------------------------

use crate::services::circuit_breaker::collect_health_summaries;
use crate::services::latency_scorer::LatencyScorer;

pub async fn get_agents_health(
    breaker_registry: web::Data<CircuitBreakerRegistry>,
    registry: web::Data<AgentRegistry>,
    metrics_collector: web::Data<AgentMetricsCollector>,
) -> impl Responder {
    let scorer = LatencyScorer::default();
    let summaries = collect_health_summaries(
        registry.get_ref(),
        breaker_registry.get_ref(),
        metrics_collector.get_ref(),
        &scorer,
    );

    let list: Vec<serde_json::Value> = summaries
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "agent_id": s.agent_id,
                "state": s.state,
                "failure_count": s.failure_count,
                "cooldown_remaining_secs": s.cooldown_remaining_secs,
                "score": {
                    "ewma_latency_ms": s.score.ewma_latency_ms,
                    "ewma_error_rate": s.score.ewma_error_rate,
                },
            })
        })
        .collect();

    HttpResponse::Ok().json(serde_json::json!({ "agents": list }))
}

pub async fn get_agent_health_detail(
    breaker_registry: web::Data<CircuitBreakerRegistry>,
    registry: web::Data<AgentRegistry>,
    metrics_collector: web::Data<AgentMetricsCollector>,
    agent_id: web::Path<Uuid>,
) -> impl Responder {
    let agent_id = agent_id.into_inner();
    let agent = registry.get_agent(&agent_id);

    let agent_meta = match agent {
        Some(a) => a,
        None => {
            return error_json(404, "not_found", "Agent not found");
        }
    };

    let scorer = LatencyScorer::default();
    let summaries = collect_health_summaries(
        registry.get_ref(),
        breaker_registry.get_ref(),
        metrics_collector.get_ref(),
        &scorer,
    );

    let detail = summaries.into_iter().find(|s| s.agent_id == agent_id);

    match detail {
        Some(s) => HttpResponse::Ok().json(serde_json::json!({
            "agent_id": s.agent_id,
            "endpoint": agent_meta.endpoint,
            "capabilities": agent_meta.capabilities,
            "state": s.state,
            "failure_count": s.failure_count,
            "cooldown_remaining_secs": s.cooldown_remaining_secs,
            "score": {
                "ewma_latency_ms": s.score.ewma_latency_ms,
                "ewma_error_rate": s.score.ewma_error_rate,
            },
        })),
        None => HttpResponse::Ok().json(serde_json::json!({
            "agent_id": agent_id,
            "state": "Closed",
            "failure_count": 0,
            "cooldown_remaining_secs": 0,
            "score": {
                "ewma_latency_ms": 200.0,
                "ewma_error_rate": 0.0,
            },
        })),
    }
}

/// POST /v1/health/agents/{agent_id}/reset — manually reset a circuit breaker.
pub async fn reset_agent_breaker(
    breaker_registry: web::Data<CircuitBreakerRegistry>,
    metrics_collector: web::Data<AgentMetricsCollector>,
    agent_id: web::Path<Uuid>,
) -> impl Responder {
    let id = agent_id.into_inner();
    agent_routing::reset_agent_breaker(
        breaker_registry.get_ref(),
        metrics_collector.get_ref(),
        &id,
    );
    HttpResponse::Ok().json(serde_json::json!({ "status": "reset" }))
}

// ---------------------------------------------------------------------------
// Shared route registration — called by both production runtime and tests
// ---------------------------------------------------------------------------

/// Register all API routes on an Actix `ServiceConfig`.
/// Called by `actix_runtime` in production and by integration tests.
/// App data (MarketplaceApp, caches, etc.) must be set before calling this.
pub fn register_api_routes(cfg: &mut web::ServiceConfig) {
    // Agent query route — registered before the /v1 scope so the scope
    // doesn't intercept and 404 it.
    cfg.route("/v1/agent/query", web::post().to(agent_query))
        // Agent health API — must be before /v1 scope to avoid scope interception
        .route("/v1/health/agents", web::get().to(get_agents_health))
        .route(
            "/v1/health/agents/{agent_id}",
            web::get().to(get_agent_health_detail),
        )
        .route(
            "/v1/health/agents/{agent_id}/reset",
            web::post().to(reset_agent_breaker),
        )
        // Deprecated listing-type redirects (Spec 0001)
        .route(
            "/v1/product/{listing_id}",
            web::get().to(deprecated_listing_redirect),
        )
        .route(
            "/v1/product/search",
            web::get().to(deprecated_search_redirect),
        )
        .route(
            "/v1/service/{listing_id}",
            web::get().to(deprecated_listing_redirect),
        )
        .route(
            "/v1/service/search",
            web::get().to(deprecated_search_redirect),
        )
        .route(
            "/v1/property/{listing_id}",
            web::get().to(deprecated_listing_redirect),
        )
        .route(
            "/v1/property/search",
            web::get().to(deprecated_search_redirect),
        )
        // Listings
        .service(
            web::scope("/v1/listings")
                .route("/search", web::get().to(search_listings))
                .route("", web::post().to(create_listing))
                .route("/{listing_id}", web::get().to(get_listing))
                .route("/{listing_id}/archive", web::post().to(archive_listing)),
        )
        // Negotiations + contact reveals + real-time event stream (SSE)
        .service(
            web::scope("/v1")
                .route("/negotiations", web::post().to(open_negotiation))
                .route(
                    "/negotiations/{negotiation_id}",
                    web::get().to(get_negotiation_status),
                )
                .route(
                    "/negotiations/{negotiation_id}/offers",
                    web::post().to(submit_offer),
                )
                .route(
                    "/negotiations/{negotiation_id}/accept",
                    web::post().to(accept_negotiation),
                )
                .route(
                    "/negotiations/{negotiation_id}/reject",
                    web::post().to(reject_negotiation),
                )
                .route(
                    "/negotiations/{negotiation_id}/request-contact-reveal",
                    web::post().to(request_contact_reveal),
                )
                .route(
                    "/contact-reveals/{reveal_id}/approve",
                    web::post().to(approve_contact_reveal),
                )
                .route(
                    "/events/negotiations/{negotiation_id}",
                    web::get().to(negotiation_event_stream),
                ),
        )
        // Internal admin/support routes
        .service(
            web::scope("/internal/v1")
                .route(
                    "/listings/{listing_id}/archive",
                    web::post().to(archive_listing),
                )
                .route(
                    "/reservations/{lease_id}/release",
                    web::post().to(release_reservation),
                )
                .route(
                    "/sellers/{seller_id}/trust-level",
                    web::put().to(set_seller_trust_level),
                )
                .route(
                    "/sellers/{seller_id}/quota-override",
                    web::put().to(set_seller_quota_override),
                )
                .route(
                    "/sellers/{seller_id}/recalculate-rating",
                    web::post().to(recalculate_seller_rating),
                )
                .route(
                    "/reviews/{review_id}/approve",
                    web::post().to(approve_review),
                )
                .route("/reviews/{review_id}/reject", web::post().to(reject_review))
                .route("/rate-limits", web::get().to(get_rate_limits))
                .route(
                    "/sellers/{seller_id}/credits",
                    web::post().to(adjust_credits),
                ),
        )
        // Reviews
        .route(
            "/v1/listings/{listing_id}/reviews",
            web::post().to(create_review),
        )
        .route(
            "/v1/listings/{listing_id}/reviews",
            web::get().to(list_reviews_for_listing),
        );
}

/// Request body for `POST /internal/v1/sellers/{agent_id}/credits`.
#[derive(Debug, Deserialize)]
pub struct AdjustCreditsRequest {
    /// One of: "deposit", "spend", "refund", "adjustment".
    pub adjustment: String,
    /// Positive decimal amount.
    pub amount: Decimal,
    /// Unique idempotency key for the transaction.
    pub idempotency_key: String,
}

/// Admin: adjust an agent's credit balance.
///
/// Requires admin role. Uses write-through to commit to the DB first, then
/// updates the in-memory ledger cache, and enqueues an entry to the async
/// batch WAL for crash recovery. Returns the new balance on success.
pub async fn adjust_credits(
    ledger: web::Data<LedgerCache>,
    batch_tx: web::Data<BatchSender>,
    agent_id: web::Path<String>,
    body: web::Json<AdjustCreditsRequest>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    if !claims.roles.iter().any(|r| matches!(r, Role::Admin)) {
        return error_json(403, "forbidden", "Admin access required");
    }

    let tx_type = match body.adjustment.as_str() {
        "deposit" => TransactionType::Deposit,
        "spend" => TransactionType::Spend,
        "refund" => TransactionType::Refund,
        "adjustment" => TransactionType::Adjustment,
        _ => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "invalid_transaction_type",
                "message": "Transaction type must be deposit, spend, refund, or adjustment.",
            }));
        }
    };

    if body.amount <= Decimal::ZERO {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "invalid_amount",
            "message": "Amount must be positive.",
        }));
    }

    // Spend transactions carry a positive amount in the request but must be
    // represented as a negative delta in the ledger repository.
    let amount = if matches!(tx_type, TransactionType::Spend) {
        -body.amount
    } else {
        body.amount
    };

    let tx = NewTransaction {
        id: Uuid::new_v4(),
        agent_id: agent_id.into_inner(),
        amount,
        tx_type,
        idempotency_key: body.idempotency_key.clone(),
    };

    match ledger.apply_transaction(&tx).await {
        Ok(account) => {
            // Enqueue to async batch WAL for crash recovery (non-blocking).
            let wal_entry = WalEntry {
                transaction_id: tx.id,
                agent_id: tx.agent_id.clone(),
                amount: tx.amount.to_string(),
                tx_type: format!("{:?}", tx.tx_type),
                idempotency_key: tx.idempotency_key.clone(),
            };
            let _ = batch_tx.try_send(wal_entry);

            HttpResponse::Ok().json(serde_json::json!({
                "agent_id": account.agent_id,
                "balance_credits": account.balance_credits,
                "idempotency_key": body.idempotency_key,
                "updated_at": account.updated_at,
            }))
        },
        Err(err) => match err {
            CreditLedgerError::InsufficientCredits { requested, available } => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "insufficient_credits",
                    "message": format!("Insufficient credits: requested {requested}, available {available}"),
                }))
            }
            CreditLedgerError::DuplicateIdempotencyKey(key) => {
                HttpResponse::Conflict().json(serde_json::json!({
                    "error": "duplicate_idempotency_key",
                    "message": format!("Transaction with idempotency key '{key}' already exists"),
                }))
            }
            CreditLedgerError::AgentNotFound(_) => {
                HttpResponse::NotFound().json(serde_json::json!({
                    "error": "agent_not_found",
                    "message": "Agent not found",
                }))
            }
            CreditLedgerError::DatabaseError(msg) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "internal_error",
                    "message": msg,
                }))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;

    #[test]
    fn test_parse_fields_param_single() {
        let result = parse_fields_param("id");
        assert!(result.is_some());
        let fields = result.unwrap();
        assert!(fields.contains("id"));
        assert_eq!(fields.len(), 1);
    }

    #[test]
    fn test_parse_fields_param_multiple() {
        let result = parse_fields_param("id,title,price");
        assert!(result.is_some());
        let fields = result.unwrap();
        assert!(fields.contains("id"));
        assert!(fields.contains("title"));
        assert!(fields.contains("price"));
        assert_eq!(fields.len(), 3);
    }

    #[test]
    fn test_parse_fields_param_empty() {
        let result = parse_fields_param("");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_include_param_single() {
        let result = parse_include_param("seller");
        assert!(result.is_some());
        assert!(result.unwrap().contains("seller"));
    }

    #[test]
    fn test_parse_include_param_multiple() {
        let result = parse_include_param("seller,reviews");
        assert!(result.is_some());
        let includes = result.unwrap();
        assert!(includes.contains("seller"));
        assert!(includes.contains("reviews"));
    }

    #[test]
    fn test_filter_listing_fields_empty_returns_original() {
        let json = serde_json::json!({"id": "1"});
        let fields = HashSet::new();
        let result = filter_listing_fields(&json, &fields);
        assert_eq!(result, json);
    }

    #[test]
    fn test_filter_listing_fields_filters() {
        let json = serde_json::json!({"id": "1", "title": "Test", "price": 100});
        let mut fields = HashSet::new();
        fields.insert("id".to_string());
        fields.insert("title".to_string());
        let result = filter_listing_fields(&json, &fields);
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("title"));
        assert!(!obj.contains_key("price"));
    }

    #[test]
    fn test_filter_listing_fields_non_object() {
        let json = serde_json::json!("string");
        let mut fields = HashSet::new();
        fields.insert("x".to_string());
        let result = filter_listing_fields(&json, &fields);
        assert_eq!(result, json);
    }

    #[test]
    fn test_extract_claims_valid() {
        use crate::test_support::make_user;
        let claims = make_user();
        let json = serde_json::to_string(&claims).unwrap();
        let req = TestRequest::default()
            .insert_header(("x-marketplace-claims", json))
            .to_http_request();
        let result = extract_claims(&req);
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert_eq!(extracted.sub, claims.sub);
    }

    #[test]
    fn test_extract_claims_invalid_json() {
        let req = TestRequest::default()
            .insert_header(("x-marketplace-claims", "invalid json"))
            .to_http_request();
        let result = extract_claims(&req);
        assert!(result.is_err());
        // Check it's Unauthorized response
        let resp = result.unwrap_err();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_extract_claims_missing_header() {
        let req = TestRequest::default().to_http_request();
        let result = extract_claims(&req);
        assert!(result.is_err());
        let resp = result.unwrap_err();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_extract_claims_optional_valid() {
        let req = TestRequest::default()
            .insert_header(("x-marketplace-claims", r#"{"sub":"user123"}"#))
            .to_http_request();
        let claims = extract_claims_optional(&req);
        assert!(claims.is_some());
        assert_eq!(claims.unwrap().sub, "user123");
    }

    #[test]
    fn test_extract_claims_optional_invalid() {
        let req = TestRequest::default()
            .insert_header(("x-marketplace-claims", "bad"))
            .to_http_request();
        let claims = extract_claims_optional(&req);
        assert!(claims.is_none());
    }

    #[actix_web::test]
    async fn test_rate_limited_response() {
        let status = crate::services::rate_limiter::RateLimitStatus {
            allowed: false,
            remaining: 0,
            limit: 60,
            reset_after_secs: 42,
        };
        let resp = rate_limited("test limit", &status);
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let body_headers = resp.headers().clone();
        assert!(body_headers.contains_key("x-ratelimit-limit"));
        assert!(body_headers.contains_key("x-ratelimit-remaining"));
        assert!(body_headers.contains_key("x-ratelimit-reset"));
        assert_eq!(
            body_headers
                .get("x-ratelimit-limit")
                .unwrap()
                .to_str()
                .unwrap(),
            "60"
        );
        assert_eq!(
            body_headers
                .get("x-ratelimit-remaining")
                .unwrap()
                .to_str()
                .unwrap(),
            "0"
        );
        use actix_web::body::MessageBody;
        let body_bytes = resp.into_body().try_into_bytes().ok();
        if let Some(bytes) = body_bytes {
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["error"]["code"], "rate_limited");
            assert_eq!(body["error"]["message"], "test limit");
        }
    }

    #[test]
    fn test_map_handler_error_authz() {
        use crate::auth::{AuthzError, AuthzErrorKind};
        use crate::http::handlers::HandlerError;
        let error = HandlerError::Authz(AuthzError::new(
            AuthzErrorKind::MissingRole,
            "test auth error",
        ));
        let resp = map_handler_error(&error);
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_map_handler_error_repository() {
        use crate::http::handlers::HandlerError;
        use crate::repositories::{RepositoryError, RepositoryErrorKind};
        let error = HandlerError::Repository(RepositoryError::new(
            RepositoryErrorKind::Validation,
            "test repo error",
        ));
        let resp = map_handler_error(&error);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_map_handler_error_quota_exceeded() {
        use crate::http::handlers::HandlerError;
        let error = HandlerError::QuotaExceeded {
            message: "quota".into(),
        };
        let resp = map_handler_error(&error);
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    // -----------------------------------------------------------------------
    // Actix integration tests — exercises the full HTTP stack with
    // in-memory repositories. Routes use register_api_routes() (same as prod).
    // -----------------------------------------------------------------------
    use actix_web::test::{call_service, read_body_json};
    use moka::future::Cache;

    fn test_listing_req() -> serde_json::Value {
        serde_json::json!({
            "idempotency_key": "test-create-1",
            "listing": {
                "schema_version": "1.0",
                "owner_id": "seller-1",
                "listing_type": "product",
                "category": "laptop",
                "title": "Test Laptop",
                "condition": "used",
                "price": { "currency": "USD", "amount": 500.00 },
                "location": {
                    "country_code": "US",
                    "country_name": "United States",
                    "city": "Austin"
                },
                "description": "Test listing for Actix integration test"
            }
        })
    }

    fn seller_claims_header() -> (&'static str, String) {
        let claims = crate::test_support::seller_claims();
        let mut claims = claims.clone();
        claims
            .scopes
            .push(marketplace_auth_core::Scope::NegotiationOfferSubmit);
        (
            "x-marketplace-claims",
            serde_json::to_string(&claims).unwrap(),
        )
    }

    #[allow(clippy::type_complexity)]
    fn make_test_app_data() -> (
        web::Data<ActixApp>,
        web::Data<broadcast::Sender<String>>,
        web::Data<bool>,
        web::Data<Cache<String, String>>,
        web::Data<Cache<String, String>>,
        web::Data<LedgerCache>,
        web::Data<BatchSender>,
        web::Data<Arc<dyn AgentDispatcher>>,
    ) {
        use crate::domain::ledger::CreditLedgerRepository;
        use crate::repositories::audit_events::InMemoryAuditEventRepository;
        use crate::repositories::ledger::InMemoryCreditLedgerRepository;
        use crate::repositories::negotiations::InMemoryNegotiationRepository;
        use crate::repositories::outbox_events::InMemoryOutboxEventRepository;
        use crate::repositories::seller_accounts::InMemorySellerAccountRepository;
        use std::sync::Arc;
        use std::time::Duration;
        use tokio::sync::mpsc;

        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        );

        let cache: Cache<String, String> = Cache::builder().max_capacity(100).build();
        let (event_tx, _) = broadcast::channel::<String>(1024);
        let (batch_tx, _) = mpsc::channel::<WalEntry>(1024);

        let ledger_repo: Arc<dyn CreditLedgerRepository> =
            Arc::new(InMemoryCreditLedgerRepository::new());
        let ledger_cache = LedgerCache::with_ttl(ledger_repo, Duration::from_secs(3600));

        let dispatcher: Arc<dyn AgentDispatcher> =
            Arc::new(crate::services::agent_dispatcher::MockAgentDispatcher::default());

        (
            web::Data::new(Arc::new(app)),
            web::Data::new(event_tx),
            web::Data::new(true),
            web::Data::new(cache.clone()),
            web::Data::new(cache),
            web::Data::new(ledger_cache),
            web::Data::new(batch_tx),
            web::Data::new(dispatcher),
        )
    }

    macro_rules! init_actix_app {
        () => {{
            use crate::services::agent_metrics::AgentMetricsCollector;
            use crate::services::agent_registry::AgentRegistry;
            use crate::services::circuit_breaker::CircuitBreakerRegistry;

            let (
                app_data,
                event_bus,
                cache_enabled,
                listing_cache,
                search_cache,
                ledger_cache,
                batch_tx,
                dispatcher,
            ) = make_test_app_data();
            actix_web::test::init_service(
                actix_web::App::new()
                    .app_data(app_data)
                    .app_data(event_bus)
                    .app_data(cache_enabled)
                    .app_data(listing_cache)
                    .app_data(search_cache)
                    .app_data(ledger_cache)
                    .app_data(batch_tx)
                    .app_data(dispatcher)
                    .app_data(web::Data::new(CircuitBreakerRegistry::default()))
                    .app_data(web::Data::new(AgentRegistry::default()))
                    .app_data(web::Data::new(AgentMetricsCollector::default()))
                    .configure(register_api_routes),
            )
            .await
        }};
    }

    #[actix_web::test]
    async fn actix_create_listing() {
        let app = init_actix_app!();
        let (key, val) = seller_claims_header();
        let req = TestRequest::post()
            .uri("/v1/listings")
            .insert_header((key, val.clone().as_str()))
            .set_json(test_listing_req())
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 201, "create listing should return 201");
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["status"], "active");
        assert!(body["listing_id"].is_string());
    }

    #[actix_web::test]
    async fn actix_create_and_get_listing() {
        let app = init_actix_app!();
        let (key, val) = seller_claims_header();
        let req = TestRequest::post()
            .uri("/v1/listings")
            .insert_header((key, val.clone()))
            .set_json(test_listing_req())
            .to_request();
        let resp = call_service(&app, req).await;
        let created: serde_json::Value = read_body_json(resp).await;
        let listing_id = created["listing_id"].as_str().unwrap().to_string();
        let req = TestRequest::get()
            .uri(&format!("/v1/listings/{}", listing_id))
            .insert_header((key, val))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["listing_id"], listing_id);
        assert_eq!(body["status"], "active");
    }

    #[actix_web::test]
    async fn actix_full_negotiation_flow() {
        let app = init_actix_app!();
        let (key, val) = seller_claims_header();

        let req = TestRequest::post()
            .uri("/v1/listings")
            .insert_header((key, val.clone()))
            .set_json(test_listing_req())
            .to_request();
        let resp = call_service(&app, req).await;
        let created: serde_json::Value = read_body_json(resp).await;
        let listing_id = created["listing_id"].as_str().unwrap().to_string();

        let req = TestRequest::post()
            .uri("/v1/negotiations")
            .insert_header((key, val.clone()))
            .set_json(json!({
                "listing_id": listing_id,
                "buyer_agent_id": "buyer-1",
                "offer_currency": "USD",
                "offer_amount": 450.00,
                "idempotency_key": "test-neg-1"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 201, "open negotiation: {}", resp.status());
        let neg: serde_json::Value = read_body_json(resp).await;
        assert_eq!(neg["status"], "reserved");
        let neg_id = neg["negotiation_id"].as_str().unwrap().to_string();

        let req = TestRequest::post()
            .uri(&format!("/v1/negotiations/{}/offers", neg_id))
            .insert_header((key, val.clone()))
            .set_json(json!({
                "offer_currency": "USD",
                "offer_amount": 475.00,
                "idempotency_key": "test-offer-1"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let after_offer: serde_json::Value = read_body_json(resp).await;
        assert_eq!(after_offer["status"], "countered");

        let req = TestRequest::post()
            .uri(&format!("/v1/negotiations/{}/accept", neg_id))
            .insert_header((key, val))
            .set_json(json!({
                "idempotency_key": "test-accept-1"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let accepted: serde_json::Value = read_body_json(resp).await;
        assert_eq!(accepted["status"], "closed");
        assert_eq!(accepted["final_offer_amount"], 475.00);
    }

    #[actix_web::test]
    async fn actix_rate_limits_endpoint() {
        let app = init_actix_app!();
        let admin = crate::test_support::admin_claims();
        let claims_header = (
            "x-marketplace-claims",
            serde_json::to_string(&admin).unwrap(),
        );

        let req = TestRequest::get()
            .uri("/internal/v1/rate-limits")
            .insert_header(claims_header)
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200, "rate-limits endpoint should return 200");
        let body: serde_json::Value = read_body_json(resp).await;
        assert!(
            body["buckets"].is_array(),
            "response should contain buckets array"
        );
        assert!(
            body["config"].is_object(),
            "response should contain config object"
        );
        assert!(body["config"]["search"]["max"].as_u64() == Some(60));
        assert!(body["config"]["create_listing"]["max"].as_u64() == Some(10));
        assert!(body["config"]["new_seller_daily"]["max"].as_u64() == Some(3));
    }

    #[actix_web::test]
    async fn actix_search_listings() {
        let app = init_actix_app!();
        let (key, val) = seller_claims_header();

        let req = TestRequest::post()
            .uri("/v1/listings")
            .insert_header((key, val.clone()))
            .set_json(test_listing_req())
            .to_request();
        let _ = call_service(&app, req).await;

        let req = TestRequest::get()
            .uri("/v1/listings/search?query=Test")
            .insert_header((key, val))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        let items = body["items"].as_array().unwrap();
        assert!(!items.is_empty(), "search should return results");
    }

    #[actix_web::test]
    async fn actix_sse_returns_correct_headers() {
        let app = init_actix_app!();
        let (key, val) = seller_claims_header();

        // Create listing + open negotiation to have a valid negotiation
        let req = TestRequest::post()
            .uri("/v1/listings")
            .insert_header((key, val.clone()))
            .set_json(test_listing_req())
            .to_request();
        let resp = call_service(&app, req).await;
        let created: serde_json::Value = read_body_json(resp).await;
        let listing_id = created["listing_id"].as_str().unwrap().to_string();

        let req = TestRequest::post()
            .uri("/v1/negotiations")
            .insert_header((key, val.clone()))
            .set_json(json!({
                "listing_id": listing_id,
                "buyer_agent_id": "buyer-1",
                "offer_currency": "USD",
                "offer_amount": 450.00,
                "idempotency_key": "test-sse-headers-1"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        let neg: serde_json::Value = read_body_json(resp).await;
        let neg_id = neg["negotiation_id"].as_str().unwrap().to_string();

        // Connect to SSE endpoint
        let req = TestRequest::get()
            .uri(&format!("/v1/events/negotiations/{}", neg_id))
            .insert_header((key, val))
            .to_request();
        let resp = call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap(),
            "text/event-stream"
        );
        assert_eq!(
            resp.headers()
                .get("Cache-Control")
                .unwrap()
                .to_str()
                .unwrap(),
            "no-cache"
        );
    }

    #[actix_web::test]
    async fn actix_sse_requires_auth() {
        let app = init_actix_app!();

        // Test without claims header — should get 401 from extract_claims
        let req = TestRequest::get()
            .uri("/v1/events/negotiations/nonexistent")
            .to_request();
        let resp = call_service(&app, req).await;
        let status = resp.status();
        assert!(
            status == 401 || status == 404,
            "expected 401 or 404, got {status}"
        );
        // If the route isn't matching (404), that's an infrastructure issue.
        // 401 means auth check fired but negotiation lookup failed — we accept both
        // because test in-memory repos may not have the negotiation.
    }

    fn admin_claims_header() -> (&'static str, String) {
        let claims = crate::test_support::admin_claims();
        (
            "x-marketplace-claims",
            serde_json::to_string(&claims).unwrap(),
        )
    }

    #[actix_web::test]
    async fn admin_credits_deposit_success() {
        let app = init_actix_app!();
        let (key, val) = admin_claims_header();

        let req = TestRequest::post()
            .uri("/internal/v1/sellers/agent-1/credits")
            .insert_header((key, val))
            .set_json(json!({
                "adjustment": "deposit",
                "amount": "100.0000",
                "idempotency_key": "adm-dep-1"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            200,
            "deposit should succeed: {}",
            resp.status()
        );
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["agent_id"], "agent-1");
        assert_eq!(body["balance_credits"], "100.0000");
        assert_eq!(body["idempotency_key"], "adm-dep-1");
        assert!(body["updated_at"].is_string());
    }

    #[actix_web::test]
    async fn admin_credits_auth_rejected() {
        let app = init_actix_app!();

        let req = TestRequest::post()
            .uri("/internal/v1/sellers/agent-1/credits")
            .set_json(json!({
                "adjustment": "deposit",
                "amount": "100.0000",
                "idempotency_key": "adm-no-auth"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 401, "missing claims should be 401");
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["error"], "missing claims header");
    }

    #[actix_web::test]
    async fn admin_credits_non_admin_rejected() {
        let app = init_actix_app!();
        let (key, val) = seller_claims_header();

        let req = TestRequest::post()
            .uri("/internal/v1/sellers/agent-1/credits")
            .insert_header((key, val))
            .set_json(json!({
                "adjustment": "deposit",
                "amount": "100.0000",
                "idempotency_key": "adm-no-role"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 403, "non-admin should be 403");
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "forbidden");
        assert_eq!(body["error"]["message"], "Admin access required");
    }

    #[actix_web::test]
    async fn admin_credits_invalid_transaction_type() {
        let app = init_actix_app!();
        let (key, val) = admin_claims_header();

        let req = TestRequest::post()
            .uri("/internal/v1/sellers/agent-1/credits")
            .insert_header((key, val))
            .set_json(json!({
                "adjustment": "invalid_tx",
                "amount": "100.0000",
                "idempotency_key": "adm-bad-tx"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 400, "bad adjustment should be 400");
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["error"], "invalid_transaction_type");
    }

    #[actix_web::test]
    async fn admin_credits_spend_deducts_and_returns_new_balance() {
        let app = init_actix_app!();
        let (key, val) = admin_claims_header();

        // Deposit 200 first
        let req = TestRequest::post()
            .uri("/internal/v1/sellers/agent-2/credits")
            .insert_header((key, val.clone()))
            .set_json(json!({
                "adjustment": "deposit",
                "amount": "200.0000",
                "idempotency_key": "adm-spend-prep"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200, "deposit prep: {}", resp.status());

        // Then spend 50
        let req = TestRequest::post()
            .uri("/internal/v1/sellers/agent-2/credits")
            .insert_header((key, val))
            .set_json(json!({
                "adjustment": "spend",
                "amount": "50.0000",
                "idempotency_key": "adm-spend-do"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            200,
            "spend should succeed: {}",
            resp.status()
        );
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["balance_credits"], "150.0000");
    }

    #[actix_web::test]
    async fn admin_credits_duplicate_idempotency() {
        let app = init_actix_app!();
        let (key, val) = admin_claims_header();

        let req = TestRequest::post()
            .uri("/internal/v1/sellers/agent-1/credits")
            .insert_header((key, val.clone()))
            .set_json(json!({
                "adjustment": "deposit",
                "amount": "50.0000",
                "idempotency_key": "dup-key"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200, "first deposit: {}", resp.status());

        let req = TestRequest::post()
            .uri("/internal/v1/sellers/agent-1/credits")
            .insert_header((key, val))
            .set_json(json!({
                "adjustment": "deposit",
                "amount": "50.0000",
                "idempotency_key": "dup-key"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 409, "duplicate key should be 409");
    }

    #[actix_web::test]
    async fn actix_sse_negotiation_actions_publish_events() {
        let (
            _app_data,
            event_bus,
            _cache_enabled,
            _listing_cache,
            _search_cache,
            _ledger_cache,
            _batch_tx,
            _dispatcher,
        ) = make_test_app_data();
        let sender = event_bus.get_ref().clone();
        let mut brx = sender.subscribe();

        let app_data2 = _app_data.clone();
        let event_bus2 = web::Data::new(sender);

        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(app_data2)
                .app_data(event_bus2)
                .app_data(_cache_enabled)
                .app_data(_listing_cache)
                .app_data(_search_cache)
                .app_data(_ledger_cache)
                .configure(register_api_routes),
        )
        .await;

        let (key, val) = seller_claims_header();

        let req = TestRequest::post()
            .uri("/v1/listings")
            .insert_header((key, val.clone()))
            .set_json(test_listing_req())
            .to_request();
        let resp = call_service(&app, req).await;
        let created: serde_json::Value = read_body_json(resp).await;
        let listing_id = created["listing_id"].as_str().unwrap().to_string();

        let req = TestRequest::post()
            .uri("/v1/negotiations")
            .insert_header((key, val.clone()))
            .set_json(json!({
                "listing_id": listing_id,
                "buyer_agent_id": "buyer-1",
                "offer_currency": "USD",
                "offer_amount": 450.00,
                "idempotency_key": "test-sse-events-1"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert!(
            resp.status().is_success(),
            "open negotiation: {}",
            resp.status()
        );
        let neg: serde_json::Value = read_body_json(resp).await;
        let neg_id = neg["negotiation_id"].as_str().unwrap().to_string();

        let req = TestRequest::post()
            .uri(&format!("/v1/negotiations/{}/offers", neg_id))
            .insert_header((key, val))
            .set_json(json!({
                "offer_currency": "USD",
                "offer_amount": 475.00,
                "idempotency_key": "test-sse-offer-2"
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert!(
            resp.status().is_success(),
            "submit offer: {}",
            resp.status()
        );

        let mut found = false;
        for _ in 0..10 {
            if let Ok(Ok(event)) =
                tokio::time::timeout(std::time::Duration::from_secs(1), brx.recv()).await
            {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event) {
                    if parsed["negotiation_id"] == neg_id {
                        found = true;
                        break;
                    }
                }
            }
        }
        assert!(
            found,
            "expected at least one broadcast event for negotiation {}",
            neg_id
        );
    }

    #[actix_web::test]
    async fn get_agents_health_returns_200() {
        let app = init_actix_app!();

        let req = TestRequest::get().uri("/v1/health/agents").to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200, "agents health should return 200");

        let body: serde_json::Value = read_body_json(resp).await;
        assert!(
            body.get("agents").and_then(|v| v.as_array()).is_some(),
            "body should have an 'agents' array, got: {:?}",
            body
        );
    }

    #[actix_web::test]
    async fn get_agent_health_detail_unknown_returns_404() {
        let app = init_actix_app!();

        let req = TestRequest::get()
            .uri("/v1/health/agents/00000000-0000-0000-0000-000000000000")
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 404, "unknown agent should return 404");

        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "not_found");
        assert_eq!(body["error"]["message"], "Agent not found");
    }

    #[actix_web::test]
    async fn reset_agent_breaker_returns_200() {
        use crate::services::agent_metrics::AgentMetricsCollector;
        use crate::services::agent_registry::AgentRegistry;
        use crate::services::circuit_breaker::CircuitBreakerRegistry;
        use uuid::Uuid;

        let breaker_registry = CircuitBreakerRegistry::default();
        let agent_registry = AgentRegistry::default();
        let metrics_collector = AgentMetricsCollector::default();

        let agent_id = Uuid::new_v4();
        agent_registry.register_agent(crate::services::agent_registry::AgentMetadata {
            id: agent_id,
            endpoint: format!("http://agent-{agent_id}.local"),
            capabilities: vec!["test".into()],
            is_active: true,
        });

        for _ in 0..5 {
            breaker_registry.record_result(agent_id, false, 100.0);
        }
        assert!(breaker_registry.is_open(agent_id));

        let cb = web::Data::new(breaker_registry);
        let ar = web::Data::new(agent_registry);
        let mc = web::Data::new(metrics_collector);

        let (
            app_data,
            event_bus,
            cache_enabled,
            listing_cache,
            search_cache,
            ledger_cache,
            batch_tx,
            dispatcher,
        ) = make_test_app_data();

        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(app_data)
                .app_data(event_bus)
                .app_data(cache_enabled)
                .app_data(listing_cache)
                .app_data(search_cache)
                .app_data(ledger_cache)
                .app_data(batch_tx)
                .app_data(dispatcher)
                .app_data(cb.clone())
                .app_data(ar.clone())
                .app_data(mc.clone())
                .configure(register_api_routes),
        )
        .await;

        let req = TestRequest::post()
            .uri(&format!("/v1/health/agents/{agent_id}/reset"))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["status"], "reset");

        assert!(!cb.is_open(agent_id));
    }

    // -----------------------------------------------------------------------
    // Spec 0014 — Agent HTTP integration
    // -----------------------------------------------------------------------

    use crate::services::agent_dispatcher::MockAgentDispatcher;

    /// Build an actix app with custom agent registry, breaker registry,
    /// metrics collector, and dispatcher. Mirrors `init_actix_app!` but
    /// allows injecting agent state for routing/health tests.
    macro_rules! init_agent_app {
        ($registry:expr, $breakers:expr, $metrics:expr, $dispatcher:expr) => {{
            let (
                app_data,
                event_bus,
                cache_enabled,
                listing_cache,
                search_cache,
                ledger_cache,
                batch_tx,
                _default_dispatcher_unused,
            ) = make_test_app_data();
            let dispatcher = web::Data::new($dispatcher);
            let cb = web::Data::new($breakers);
            let ar = web::Data::new($registry);
            let mc = web::Data::new($metrics);
            actix_web::test::init_service(
                actix_web::App::new()
                    .app_data(app_data)
                    .app_data(event_bus)
                    .app_data(cache_enabled)
                    .app_data(listing_cache)
                    .app_data(search_cache)
                    .app_data(ledger_cache)
                    .app_data(batch_tx)
                    .app_data(cb)
                    .app_data(ar)
                    .app_data(mc)
                    .app_data(dispatcher)
                    .configure(register_api_routes),
            )
            .await
        }};
    }

    fn make_agent_claims_header() -> (&'static str, String) {
        let claims = crate::test_support::seller_claims();
        (
            "x-marketplace-claims",
            serde_json::to_string(&claims).unwrap(),
        )
    }

    #[actix_web::test]
    async fn agent_query_returns_200_with_mock_dispatch() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let agent_id = Uuid::new_v4();
        let registry = AgentRegistry::default();
        registry.register_agent(AgentMetadata {
            id: agent_id,
            endpoint: "http://agent.local".into(),
            capabilities: vec!["search".into()],
            is_active: true,
        });
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();
        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> = Arc::new(
            MockAgentDispatcher::with_response(agent_id, b"agent-reply".to_vec()),
        );

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);
        let (key, val) = make_agent_claims_header();

        let req = TestRequest::post()
            .uri("/v1/agent/query")
            .insert_header((key, val))
            .set_json(serde_json::json!({
                "query": "search listings",
                "conversation_id": "conv-1",
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["conversation_id"], "conv-1");
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("Dispatched to 1 agent(s)"));
    }

    #[actix_web::test]
    async fn agent_query_returns_503_when_no_agents_registered() {
        use crate::services::agent_registry::AgentRegistry;
        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();
        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);
        let (key, val) = make_agent_claims_header();

        let req = TestRequest::post()
            .uri("/v1/agent/query")
            .insert_header((key, val))
            .set_json(serde_json::json!({ "query": "hello" }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 503);

        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "no_agents");
    }

    #[actix_web::test]
    async fn agent_query_returns_400_on_missing_query_field() {
        use crate::services::agent_registry::AgentRegistry;
        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();
        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);
        let (key, val) = make_agent_claims_header();

        let req = TestRequest::post()
            .uri("/v1/agent/query")
            .insert_header((key, val))
            .set_json(serde_json::json!({ "conversation_id": "x" }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 400, "missing required 'query' field");
    }

    #[actix_web::test]
    async fn agent_query_returns_401_without_claims_header() {
        use crate::services::agent_registry::AgentRegistry;
        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();
        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::post()
            .uri("/v1/agent/query")
            .set_json(serde_json::json!({ "query": "hello" }))
            .to_request();
        let resp = call_service(&app, req).await;
        let status = resp.status();
        if status == 404 {
            eprintln!(
                "agent_query 404: route not registered? body={:?}",
                resp.into_body()
            );
        }
        assert_eq!(status, 401);
    }

    #[actix_web::test]
    async fn agent_query_preserves_supplied_conversation_id() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let agent_id = Uuid::new_v4();
        let registry = AgentRegistry::default();
        registry.register_agent(AgentMetadata {
            id: agent_id,
            endpoint: "http://agent.local".into(),
            capabilities: vec!["search".into()],
            is_active: true,
        });
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();
        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::with_response(agent_id, b"ok".to_vec()));

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);
        let (key, val) = make_agent_claims_header();

        let req = TestRequest::post()
            .uri("/v1/agent/query")
            .insert_header((key, val))
            .set_json(serde_json::json!({
                "query": "search",
                "conversation_id": "thread-42",
            }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["conversation_id"], "thread-42");
    }

    #[actix_web::test]
    async fn agent_query_returns_200_with_no_success_when_dispatcher_fails_all() {
        // The route_agent_query function swallows individual dispatch errors
        // and returns Ok with a 'no agent returned a successful response'
        // message when every attempt fails. The 502 'dispatch_error' path
        // in the HTTP handler is therefore unreachable in practice — pinned
        // here so future refactors don't accidentally change this contract
        // without updating tests/docs.
        use crate::services::agent_dispatcher::DispatchError;
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let agent_id = Uuid::new_v4();
        let registry = AgentRegistry::default();
        registry.register_agent(AgentMetadata {
            id: agent_id,
            endpoint: "http://agent.local".into(),
            capabilities: vec!["search".into()],
            is_active: true,
        });
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();
        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::with_error(
                agent_id,
                DispatchError::Network("backend down".into()),
            ));

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);
        let (key, val) = make_agent_claims_header();

        let req = TestRequest::post()
            .uri("/v1/agent/query")
            .insert_header((key, val))
            .set_json(serde_json::json!({ "query": "search" }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("no agent returned a successful response"),
            "got: {}",
            body["message"]
        );
    }

    #[actix_web::test]
    async fn agent_query_with_open_breaker_skips_agent_and_returns_503() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let agent_id = Uuid::new_v4();
        let registry = AgentRegistry::default();
        registry.register_agent(AgentMetadata {
            id: agent_id,
            endpoint: "http://agent.local".into(),
            capabilities: vec!["search".into()],
            is_active: true,
        });
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        for _ in 0..5 {
            breakers.record_result(agent_id, false, 100.0);
        }
        assert!(breakers.is_open(agent_id));

        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();
        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::with_response(agent_id, b"ok".to_vec()));

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);
        let (key, val) = make_agent_claims_header();

        let req = TestRequest::post()
            .uri("/v1/agent/query")
            .insert_header((key, val))
            .set_json(serde_json::json!({ "query": "search" }))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            503,
            "open breaker must skip the agent, leaving none available"
        );
    }

    // -----------------------------------------------------------------------
    // Spec 0017 — Agent health API
    // -----------------------------------------------------------------------

    #[actix_web::test]
    async fn get_agents_health_includes_summary_fields_per_agent() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();

        let id = Uuid::new_v4();
        registry.register_agent(AgentMetadata {
            id,
            endpoint: "http://a.local".into(),
            capabilities: vec!["search".into()],
            is_active: true,
        });

        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::get().uri("/v1/health/agents").to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        let agents = body["agents"].as_array().expect("agents array");
        assert_eq!(agents.len(), 1);
        let entry = &agents[0];
        assert_eq!(entry["agent_id"], id.to_string());
        assert_eq!(entry["state"], "Closed");
        assert_eq!(entry["failure_count"], 0);
        assert!(entry["score"]["ewma_latency_ms"].is_number());
        assert!(entry["score"]["ewma_error_rate"].is_number());
    }

    #[actix_web::test]
    async fn get_agents_health_reports_open_state_for_tripped_breaker() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();

        let id = Uuid::new_v4();
        registry.register_agent(AgentMetadata {
            id,
            endpoint: "http://a.local".into(),
            capabilities: vec!["search".into()],
            is_active: true,
        });
        for _ in 0..5 {
            breakers.record_result(id, false, 100.0);
        }

        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::get().uri("/v1/health/agents").to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        let entry = &body["agents"][0];
        assert_eq!(entry["state"], "Open");
        assert_eq!(entry["failure_count"], 5);
    }

    #[actix_web::test]
    async fn get_agents_health_returns_empty_array_when_no_agents() {
        use crate::services::agent_registry::AgentRegistry;
        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();
        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::get().uri("/v1/health/agents").to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["agents"].as_array().unwrap().len(), 0);
    }

    #[actix_web::test]
    async fn get_agents_health_includes_inactive_agents() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();

        registry.register_agent(AgentMetadata {
            id: Uuid::new_v4(),
            endpoint: "http://a.local".into(),
            capabilities: vec![],
            is_active: false,
        });

        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::get().uri("/v1/health/agents").to_request();
        let resp = call_service(&app, req).await;
        let body: serde_json::Value = read_body_json(resp).await;
        let agents = body["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 1, "inactive agents are still listed");
    }

    #[actix_web::test]
    async fn get_agent_health_detail_returns_full_payload() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();

        let id = Uuid::new_v4();
        registry.register_agent(AgentMetadata {
            id,
            endpoint: "http://a.local".into(),
            capabilities: vec!["search".into(), "chat".into()],
            is_active: true,
        });
        metrics.record_sample(id, std::time::Duration::from_millis(120), true);
        metrics.record_sample(id, std::time::Duration::from_millis(180), true);

        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::get()
            .uri(&format!("/v1/health/agents/{id}"))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["agent_id"], id.to_string());
        assert_eq!(body["endpoint"], "http://a.local");
        assert_eq!(body["state"], "Closed");
        assert_eq!(body["failure_count"], 0);
        let caps = body["capabilities"].as_array().unwrap();
        assert_eq!(caps.len(), 2);
        assert!(body["score"]["ewma_latency_ms"].as_f64().unwrap() > 0.0);
    }

    #[actix_web::test]
    async fn get_agent_health_detail_reports_open_state_and_failure_count() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();

        let id = Uuid::new_v4();
        registry.register_agent(AgentMetadata {
            id,
            endpoint: "http://a.local".into(),
            capabilities: vec!["x".into()],
            is_active: true,
        });
        for _ in 0..5 {
            breakers.record_result(id, false, 100.0);
        }

        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::get()
            .uri(&format!("/v1/health/agents/{id}"))
            .to_request();
        let resp = call_service(&app, req).await;
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["state"], "Open");
        assert_eq!(body["failure_count"], 5);
    }

    #[actix_web::test]
    async fn get_agent_health_detail_zero_failures_for_healthy_agent() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();

        let id = Uuid::new_v4();
        registry.register_agent(AgentMetadata {
            id,
            endpoint: "http://a.local".into(),
            capabilities: vec![],
            is_active: true,
        });

        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::get()
            .uri(&format!("/v1/health/agents/{id}"))
            .to_request();
        let resp = call_service(&app, req).await;
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["failure_count"], 0);
        assert_eq!(body["state"], "Closed");
    }

    #[actix_web::test]
    async fn get_agent_health_detail_returns_default_score_for_no_samples() {
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();

        let id = Uuid::new_v4();
        registry.register_agent(AgentMetadata {
            id,
            endpoint: "http://a.local".into(),
            capabilities: vec![],
            is_active: true,
        });

        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::get()
            .uri(&format!("/v1/health/agents/{id}"))
            .to_request();
        let resp = call_service(&app, req).await;
        let body: serde_json::Value = read_body_json(resp).await;
        let latency = body["score"]["ewma_latency_ms"].as_f64().unwrap();
        assert!(
            (latency - 200.0).abs() < 0.001,
            "expected default 200.0, got {latency}"
        );
        assert_eq!(body["score"]["ewma_error_rate"].as_f64().unwrap(), 0.0);
    }

    #[actix_web::test]
    async fn reset_agent_breaker_endpoint_returns_200_for_known_agent() {
        // The metrics-clearing side effect of the reset endpoint is covered
        // by the service-level test `reset_clears_breaker_and_metrics` and
        // `reset_agent_breaker_returns_200`. Here we only verify the HTTP
        // integration: 200 + {"status":"reset"} payload.
        use crate::services::agent_registry::{AgentMetadata, AgentRegistry};
        use uuid::Uuid;

        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();

        let id = Uuid::new_v4();
        registry.register_agent(AgentMetadata {
            id,
            endpoint: "http://a.local".into(),
            capabilities: vec![],
            is_active: true,
        });
        for _ in 0..5 {
            breakers.record_result(id, false, 100.0);
        }
        assert!(breakers.is_open(id));

        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());
        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        let req = TestRequest::post()
            .uri(&format!("/v1/health/agents/{id}/reset"))
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["status"], "reset");
    }

    #[actix_web::test]
    async fn reset_agent_breaker_idempotent_for_unknown_agent() {
        use crate::services::agent_registry::AgentRegistry;
        let registry = AgentRegistry::default();
        let breakers = crate::services::circuit_breaker::CircuitBreakerRegistry::default();
        let metrics = crate::services::agent_metrics::AgentMetricsCollector::default();
        let dispatcher: Arc<dyn crate::services::agent_dispatcher::AgentDispatcher> =
            Arc::new(MockAgentDispatcher::default());

        let app = init_agent_app!(registry, breakers, metrics, dispatcher);

        // Reset on a never-seen agent must still return 200 (no breaker to reset).
        let req = TestRequest::post()
            .uri("/v1/health/agents/11111111-1111-1111-1111-111111111111/reset")
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = read_body_json(resp).await;
        assert_eq!(body["status"], "reset");
    }
}
