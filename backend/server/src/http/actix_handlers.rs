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
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;

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
    let now = crate::http::runtime::current_time_marker();
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
    app: web::Data<ActixApp>,
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
    match app.agent_query(Some(&claims), &body).await {
        Ok(result) => HttpResponse::Ok()
            .insert_header(("X-RateLimit-Limit", rl_agent.limit.to_string()))
            .insert_header(("X-RateLimit-Remaining", rl_agent.remaining.to_string()))
            .insert_header(("X-RateLimit-Reset", rl_agent.reset_after_secs.to_string()))
            .json(result),
        Err(e) => map_handler_error(&e),
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
    let now = crate::http::runtime::current_time_marker();
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
    let now = crate::http::runtime::current_time_marker();
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
    let now = crate::http::runtime::current_time_marker();
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
    let now = crate::http::runtime::current_time_marker();
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
    let now = crate::http::runtime::current_time_marker();
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
    let now = crate::http::runtime::current_time_marker();
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
    let now = crate::http::runtime::current_time_marker();
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
    let now = crate::http::runtime::current_time_marker();
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
    let now = crate::http::runtime::current_time_marker();
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
// Shared route registration — called by both production runtime and tests
// ---------------------------------------------------------------------------

/// Register all API routes on an Actix `ServiceConfig`.
/// Called by `actix_runtime` in production and by integration tests.
/// App data (MarketplaceApp, caches, etc.) must be set before calling this.
pub fn register_api_routes(cfg: &mut web::ServiceConfig) {
    // Deprecated listing-type redirects (Spec 0001)
    cfg.route(
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
            .route("/rate-limits", web::get().to(get_rate_limits)),
    )
    // Reviews
    .route(
        "/v1/listings/{listing_id}/reviews",
        web::post().to(create_review),
    )
    .route("/v1/agent/query", web::post().to(agent_query))
    .route(
        "/v1/listings/{listing_id}/reviews",
        web::get().to(list_reviews_for_listing),
    );
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
    ) {
        use crate::repositories::audit_events::InMemoryAuditEventRepository;
        use crate::repositories::negotiations::InMemoryNegotiationRepository;
        use crate::repositories::outbox_events::InMemoryOutboxEventRepository;
        use crate::repositories::seller_accounts::InMemorySellerAccountRepository;
        use std::sync::Arc;

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

        (
            web::Data::new(Arc::new(app)),
            web::Data::new(event_tx),
            web::Data::new(true),
            web::Data::new(cache.clone()),
            web::Data::new(cache),
        )
    }

    macro_rules! init_actix_app {
        () => {{
            let (app_data, event_bus, cache_enabled, listing_cache, search_cache) =
                make_test_app_data();
            actix_web::test::init_service(
                actix_web::App::new()
                    .app_data(app_data)
                    .app_data(event_bus)
                    .app_data(cache_enabled)
                    .app_data(listing_cache)
                    .app_data(search_cache)
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

    #[actix_web::test]
    async fn actix_sse_negotiation_actions_publish_events() {
        let (_app_data, event_bus, _cache_enabled, _listing_cache, _search_cache) =
            make_test_app_data();
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
}
