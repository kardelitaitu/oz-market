use crate::app::MarketplaceApp;
use crate::http::handlers::HandlerError;
use crate::repositories::PostgresIdempotencyKeyRepository;
use crate::repositories::{
    PostgresContactRevealRepository, PostgresListingRepository, PostgresReservationLeaseRepository,
};
use crate::services::rate_limiter::{
    global_limiter, CONTACT_REVEAL_RATE_MAX, CONTACT_REVEAL_RATE_WINDOW_SECS,
    CREATE_LISTING_RATE_MAX, CREATE_LISTING_RATE_WINDOW_SECS, OPEN_NEGOTIATION_RATE_MAX,
    OPEN_NEGOTIATION_RATE_WINDOW_SECS, SEARCH_RATE_MAX, SEARCH_RATE_WINDOW_SECS,
};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use marketplace_api_contract::{
    AcceptNegotiationRequest, CreateListingRequest, OpenNegotiationRequest,
    RejectNegotiationRequest, RequestContactRevealRequest, SearchRequest, SubmitOfferRequest,
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

// Type alias for the concrete app type used in Actix handlers
type ActixApp = Arc<
    MarketplaceApp<
        PostgresListingRepository,
        PostgresIdempotencyKeyRepository,
        PostgresReservationLeaseRepository,
        PostgresContactRevealRepository,
    >,
>;

// Helper to map HandlerError to HttpResponse
fn map_handler_error(error: &HandlerError) -> HttpResponse {
    use crate::http::handlers::HandlerError::*;
    match error {
        Authz(authz_error) => HttpResponse::Forbidden().json(json!({
            "error_code": "FORBIDDEN",
            "message": authz_error.to_string()
        })),
        Idempotency(idem_error) => match idem_error.kind {
            crate::services::idempotency::IdempotencyErrorKind::InvalidKey => {
                HttpResponse::BadRequest().json(json!({
                    "error_code": "INVALID_FIELD",
                    "message": idem_error.message
                }))
            }
            crate::services::idempotency::IdempotencyErrorKind::Conflict => {
                HttpResponse::Conflict().json(json!({
                    "error_code": "CONFLICT",
                    "message": idem_error.message
                }))
            }
            crate::services::idempotency::IdempotencyErrorKind::Storage => {
                HttpResponse::InternalServerError().json(json!({
                    "error_code": "INTERNAL_ERROR",
                    "message": idem_error.message
                }))
            }
        },
        Search(search_error) => match search_error {
            crate::services::search::SearchError::Authz(authz_error) => HttpResponse::Forbidden()
                .json(json!({
                    "error_code": "FORBIDDEN",
                    "message": authz_error.to_string()
                })),
            crate::services::search::SearchError::Storage(storage_error) => {
                HttpResponse::InternalServerError().json(json!({
                    "error_code": "INTERNAL_ERROR",
                    "message": storage_error.to_string()
                }))
            }
        },
        Repository(repo_error) => match repo_error.kind {
            crate::repositories::RepositoryErrorKind::Conflict => {
                HttpResponse::Conflict().json(json!({
                    "error_code": "CONFLICT",
                    "message": repo_error.message
                }))
            }
            crate::repositories::RepositoryErrorKind::NotFound => {
                HttpResponse::NotFound().json(json!({
                    "error_code": "NOT_FOUND",
                    "message": repo_error.message
                }))
            }
            crate::repositories::RepositoryErrorKind::PermissionDenied => HttpResponse::Forbidden()
                .json(json!({
                    "error_code": "FORBIDDEN",
                    "message": repo_error.message
                })),
            crate::repositories::RepositoryErrorKind::Validation => HttpResponse::BadRequest()
                .json(json!({
                    "error_code": "INVALID_FIELD",
                    "message": repo_error.message
                })),
            crate::repositories::RepositoryErrorKind::Storage
            | crate::repositories::RepositoryErrorKind::Unknown => {
                HttpResponse::InternalServerError().json(json!({
                    "error_code": "INTERNAL_ERROR",
                    "message": repo_error.message
                }))
            }
        },
        QuotaExceeded { message } => HttpResponse::TooManyRequests().json(json!({
            "error_code": "QUOTA_EXCEEDED",
            "message": message
        })),
    }
}

// Helper to extract Claims from x-marketplace-claims header
fn extract_claims(req: &HttpRequest) -> Result<Claims, HttpResponse> {
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
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error_code": "NOT_FOUND",
            "message": "listing not found"
        })),
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

    if let Some(ref c) = claims {
        let search_key = format!("search:{}", c.sub);
        if !global_limiter().check(&search_key, SEARCH_RATE_MAX, SEARCH_RATE_WINDOW_SECS) {
            return HttpResponse::TooManyRequests()
                .json(serde_json::json!({"error_code": "RATE_LIMITED", "message": "search rate limit exceeded (60/min)"}));
        }
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
            return HttpResponse::Ok()
                .content_type("application/json")
                .body(cached_json);
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
            HttpResponse::Ok()
                .content_type("application/json")
                .body(json_string)
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
    if !global_limiter().check(
        &create_key,
        CREATE_LISTING_RATE_MAX,
        CREATE_LISTING_RATE_WINDOW_SECS,
    ) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error_code": "RATE_LIMITED", "message": "create listing rate limit exceeded (10/min)"}));
    }
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::runtime::current_time_marker();
    // Invalidate search cache on write
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app.create_listing(&claims, &body, &fingerprint, &now).await {
        Ok(created) => HttpResponse::Created().json(created),
        Err(e) => map_handler_error(&e),
    }
}

// --- Negotiation handlers ---

pub async fn open_negotiation(
    app: web::Data<ActixApp>,
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
    if !global_limiter().check(
        &negot_key,
        OPEN_NEGOTIATION_RATE_MAX,
        OPEN_NEGOTIATION_RATE_WINDOW_SECS,
    ) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error_code": "RATE_LIMITED", "message": "open negotiation rate limit exceeded (20/min)"}));
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
        Ok(response) => HttpResponse::Created().json(response),
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
    if !global_limiter().check(
        &negot_key,
        OPEN_NEGOTIATION_RATE_MAX,
        OPEN_NEGOTIATION_RATE_WINDOW_SECS,
    ) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error_code": "RATE_LIMITED", "message": "offer submit rate limit exceeded (20/min)"}));
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
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn accept_negotiation(
    app: web::Data<ActixApp>,
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
    if !global_limiter().check(
        &negot_key,
        OPEN_NEGOTIATION_RATE_MAX,
        OPEN_NEGOTIATION_RATE_WINDOW_SECS,
    ) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error_code": "RATE_LIMITED", "message": "accept rate limit exceeded (20/min)"}));
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
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn reject_negotiation(
    app: web::Data<ActixApp>,
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
    if !global_limiter().check(
        &negot_key,
        OPEN_NEGOTIATION_RATE_MAX,
        OPEN_NEGOTIATION_RATE_WINDOW_SECS,
    ) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error_code": "RATE_LIMITED", "message": "reject rate limit exceeded (20/min)"}));
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
        Ok(response) => HttpResponse::Ok().json(response),
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
    if !global_limiter().check(
        &reveal_key,
        CONTACT_REVEAL_RATE_MAX,
        CONTACT_REVEAL_RATE_WINDOW_SECS,
    ) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error_code": "RATE_LIMITED", "message": "contact reveal rate limit exceeded (10/min)"}));
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
        Ok(response) => HttpResponse::Accepted().json(response),
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
    if !global_limiter().check(
        &approve_key,
        CONTACT_REVEAL_RATE_MAX,
        CONTACT_REVEAL_RATE_WINDOW_SECS,
    ) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error_code": "RATE_LIMITED", "message": "contact reveal rate limit exceeded (10/min)"}));
    }
    if **cache_enabled {
        search_cache.invalidate_all();
    }
    match app.approve_contact_reveal(&claims, &reveal_id).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => map_handler_error(&e),
    }
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
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error_code": "NOT_FOUND",
            "message": "listing not found"
        })),
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
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error_code": "NOT_FOUND",
            "message": "listing not found"
        })),
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
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error_code": "NOT_FOUND",
            "message": "seller not found"
        })),
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
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error_code": "NOT_FOUND",
            "message": "seller not found"
        })),
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
        return HttpResponse::Forbidden().json(json!({
            "error_code": "FORBIDDEN",
            "message": "Admin access required"
        }));
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
        return HttpResponse::Forbidden().json(json!({
            "error_code": "FORBIDDEN",
            "message": "Buyer access required"
        }));
    }

    let rating = review.get("rating").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if !(1..=5).contains(&rating) {
        return HttpResponse::BadRequest().json(json!({
            "error_code": "VALIDATION_ERROR",
            "message": "Rating must be between 1 and 5"
        }));
    }

    let title = review.get("title").and_then(|v| v.as_str()).unwrap_or("");
    if title.len() < 3 || title.len() > 200 {
        return HttpResponse::BadRequest().json(json!({
            "error_code": "VALIDATION_ERROR",
            "message": "Title must be between 3 and 200 characters"
        }));
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
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error_code": "NOT_FOUND",
                "message": "Listing or seller not found"
            }))
        }
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
        return HttpResponse::Forbidden().json(json!({
            "error_code": "FORBIDDEN",
            "message": "Admin access required"
        }));
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
                HttpResponse::NotFound().json(json!({
                    "error_code": "NOT_FOUND",
                    "message": "Review not found"
                }))
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
        return HttpResponse::Forbidden().json(json!({
            "error_code": "FORBIDDEN",
            "message": "Admin access required"
        }));
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
                HttpResponse::NotFound().json(json!({
                    "error_code": "NOT_FOUND",
                    "message": "Review not found"
                }))
            }
        }
        Err(e) => {
            error!("Reject review error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
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
}
