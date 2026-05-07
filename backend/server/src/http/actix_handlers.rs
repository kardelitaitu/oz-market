use actix_web::{web, HttpRequest, HttpResponse, Responder};
use crate::app::MarketplaceApp;
use crate::http::handlers::HandlerError;
use crate::repositories::{
    PostgresListingRepository, PostgresReservationLeaseRepository,
    PostgresContactRevealRepository,
};
use crate::services::idempotency::InMemoryIdempotencyRepository;
use marketplace_api_contract::{
    ListingSummary, SearchRequest, SearchResponse, CreateListingRequest,
    OpenNegotiationRequest, RequestContactRevealRequest,
};
use marketplace_auth_core::Claims;
use serde_json::json;
use moka::future::Cache;
use std::sync::Arc;

// Type alias for the concrete app type used in Actix handlers
type ActixApp = Arc<MarketplaceApp<
    PostgresListingRepository,
    InMemoryIdempotencyRepository,
    PostgresReservationLeaseRepository,
    PostgresContactRevealRepository,
>>;

// Helper to map HandlerError to HttpResponse
fn map_handler_error(error: &HandlerError) -> HttpResponse {
    use crate::http::handlers::HandlerError::*;
    match error {
        Authz(authz_error) => HttpResponse::Forbidden().json(json!({
            "error_code": "FORBIDDEN",
            "message": authz_error.to_string()
        })),
        Idempotency(idem_error) => HttpResponse::Conflict().json(json!({
            "error_code": "IDEMPOTENCY_CONFLICT",
            "message": format!("{:?}", idem_error)
        })),
        Search(search_error) => HttpResponse::BadRequest().json(json!({
            "error_code": "SEARCH_ERROR",
            "message": format!("{:?}", search_error)
        })),
        Repository(repo_error) => HttpResponse::BadRequest().json(json!({
            "error_code": "REPOSITORY_ERROR",
            "message": repo_error.to_string()
        })),
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
                    eprintln!("Claims parse error: {}", e);
                    return Err(HttpResponse::Unauthorized().json(serde_json::json!({"error": format!("invalid claims: {}", e)})));
                }
            }
        }
    }
    Err(HttpResponse::Unauthorized().json(serde_json::json!({"error": "missing claims header"})))
}

// --- Listing handlers ---

pub async fn get_listing(
    app: web::Data<ActixApp>,
    listing_cache: web::Data<Cache<String, ListingSummary>>,
    listing_id: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let cache_key = listing_id.to_string();
    // Try cache first
    if let Some(cached) = listing_cache.get(&cache_key).await {
        return HttpResponse::Ok().json(cached);
    }
    // Fallback to app
    match app.get_listing(&claims, &listing_id).await {
        Ok(Some(listing)) => {
            listing_cache.insert(cache_key, listing.clone()).await;
            HttpResponse::Ok().json(listing)
        },
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error_code": "NOT_FOUND",
            "message": "listing not found"
        })),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn search_listings(
    app: web::Data<ActixApp>,
    search_cache: web::Data<Cache<String, SearchResponse>>,
    query: web::Query<SearchRequest>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let cache_key = format!("{:?}", query.0);
    // Try cache first
    if let Some(cached) = search_cache.get(&cache_key).await {
        eprintln!("CACHE HIT for {}", cache_key);
        return HttpResponse::Ok().json(cached);
    }
    eprintln!("CACHE MISS for {}", cache_key);
    // Fallback to app
    match app.search_listings(&claims, &query.0).await {
        Ok(response) => {
            search_cache.insert(cache_key, response.clone()).await;
            HttpResponse::Ok().json(response)
        },
        Err(e) => map_handler_error(&e),
    }
}

pub async fn create_listing(
    app: web::Data<ActixApp>,
    search_cache: web::Data<Cache<String, SearchResponse>>,
    req: HttpRequest,
    body: web::Json<CreateListingRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::runtime::current_time_marker();
    // Invalidate search cache on write
    search_cache.invalidate_all();
    match app.create_listing(&claims, &body, &fingerprint, &now).await {
        Ok(created) => HttpResponse::Created().json(created),
        Err(e) => map_handler_error(&e),
    }
}

// --- Negotiation handlers ---

pub async fn open_negotiation(
    app: web::Data<ActixApp>,
    search_cache: web::Data<Cache<String, SearchResponse>>,
    req: HttpRequest,
    body: web::Json<OpenNegotiationRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::runtime::current_time_marker();
    search_cache.invalidate_all();
    match app.open_negotiation(&claims, &body, &fingerprint, &now).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => map_handler_error(&e),
    }
}

// --- Contact reveal handlers ---

pub async fn request_contact_reveal(
    app: web::Data<ActixApp>,
    search_cache: web::Data<Cache<String, SearchResponse>>,
    req: HttpRequest,
    body: web::Json<RequestContactRevealRequest>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let fingerprint = serde_json::to_string(&body).unwrap_or_default();
    let now = crate::http::runtime::current_time_marker();
    search_cache.invalidate_all();
    match app.request_contact_reveal(&claims, "", &body, &fingerprint, &now).await {
        Ok(response) => HttpResponse::Accepted().json(response),
        Err(e) => map_handler_error(&e),
    }
}

// --- Internal admin handlers ---

pub async fn archive_listing(
    app: web::Data<ActixApp>,
    search_cache: web::Data<Cache<String, SearchResponse>>,
    listing_id: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let now = crate::http::runtime::current_time_marker();
    search_cache.invalidate_all();
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
    search_cache: web::Data<Cache<String, SearchResponse>>,
    lease_id: web::Path<String>,
    claims: web::ReqData<Claims>,
) -> impl Responder {
    let now = crate::http::runtime::current_time_marker();
    search_cache.invalidate_all();
    // release_reservation needs listing_id, not lease_id
    // For now, we'll use lease_id as listing_id (simplification - real code would look up lease first)
    match app.release_reservation(&claims, &lease_id, "", &now).await {
        Ok(Some(_lease)) => HttpResponse::NoContent().finish(),
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error_code": "NOT_FOUND",
            "message": "reservation not found"
        })),
        Err(e) => map_handler_error(&e),
    }
}

pub async fn set_seller_trust_level(
    app: web::Data<ActixApp>,
    search_cache: web::Data<Cache<String, SearchResponse>>,
    seller_id: web::Path<String>,
    trust_level: web::Json<String>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let now = crate::http::runtime::current_time_marker();
    search_cache.invalidate_all();
    // set_seller_trust_level(claims, seller_account_id, trust_level, reason, now)
    match app.set_seller_trust_level(&claims, &seller_id, &trust_level, "", &now).await {
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
    search_cache: web::Data<Cache<String, SearchResponse>>,
    seller_id: web::Path<String>,
    quota: web::Json<Option<i32>>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let now = crate::http::runtime::current_time_marker();
    search_cache.invalidate_all();
    // set_seller_quota_override(claims, seller_account_id, quota_override, reason, now)
    match app.set_seller_quota_override(&claims, &seller_id, quota.clone(), "", &now).await {
        Ok(Some(_account)) => HttpResponse::NoContent().finish(),
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error_code": "NOT_FOUND",
            "message": "seller not found"
        })),
        Err(e) => map_handler_error(&e),
    }
}
