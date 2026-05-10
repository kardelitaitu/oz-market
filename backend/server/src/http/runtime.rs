use crate::app::MarketplaceApp;
use crate::observability::ServerObservability;
#[cfg(not(test))]
use crate::repositories::audit_events::PostgresAuditEventRepository;
#[cfg(not(test))]
use crate::repositories::contact_reveals::PostgresContactRevealRepository;
#[cfg(not(test))]
use crate::repositories::listings::PostgresListingRepository;
#[cfg(not(test))]
use crate::repositories::outbox_events::PostgresOutboxEventRepository;
#[cfg(not(test))]
use crate::repositories::reservations::PostgresReservationLeaseRepository;
#[cfg(not(test))]
use crate::repositories::seller_accounts::PostgresSellerAccountRepository;
use crate::repositories::{
    ContactRevealRepository, IdempotencyKeyRepository, ListingRepository,
    ReservationLeaseRepository,
};
use crate::services::idempotency::InMemoryIdempotencyRepository;
use marketplace_api_contract::{
    ApiErrorCode, Category, Condition, CreateListingRequest, ListingStatus, OpenNegotiationRequest,
    RequestContactRevealRequest, SearchLocationFilter, SearchPriceFilter, SearchRequest,
    SearchSort,
};
use marketplace_auth_core::{Claims, Role};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[cfg(not(test))]
type ProductionRuntimeApp = MarketplaceApp<
    PostgresListingRepository,
    InMemoryIdempotencyRepository,
    PostgresReservationLeaseRepository,
    PostgresContactRevealRepository,
>;

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    #[cfg(not(test))]
    {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async_run())
    }

    #[cfg(test)]
    {
        Ok(())
    }
}

#[cfg(not(test))]
async fn async_run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let bind = std::env::var("MARKETPLACE_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = TcpListener::bind(&bind).await?;
    let (pool, audit_repo, outbox_repo) = build_repositories().await?;
    let observability = Arc::new(ServerObservability::new());
    let app = build_runtime_app(pool, audit_repo, outbox_repo);

    loop {
        let (stream, _) = listener.accept().await?;
        let app = Arc::clone(&app);
        let observability = Arc::clone(&observability);
        tokio::spawn(async move {
            let _ = handle_connection(stream, app, observability).await;
        });
    }
}

#[cfg(not(test))]
async fn build_repositories() -> Result<
    (
        sqlx::postgres::PgPool,
        Arc<dyn crate::repositories::AuditEventRepository>,
        Arc<dyn crate::repositories::OutboxEventRepository>,
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
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let audit_repo: Arc<dyn crate::repositories::AuditEventRepository> =
        Arc::new(PostgresAuditEventRepository::new(pool.clone()));
    let outbox_repo: Arc<dyn crate::repositories::OutboxEventRepository> =
        Arc::new(PostgresOutboxEventRepository::new(pool.clone()));
    Ok((pool, audit_repo, outbox_repo))
}

#[cfg(not(test))]
fn build_runtime_app(
    pool: sqlx::postgres::PgPool,
    audit_repo: Arc<dyn crate::repositories::AuditEventRepository>,
    outbox_repo: Arc<dyn crate::repositories::OutboxEventRepository>,
) -> Arc<ProductionRuntimeApp> {
    Arc::new(ProductionRuntimeApp::new(
        PostgresListingRepository::new(pool.clone()),
        InMemoryIdempotencyRepository::new(),
        PostgresReservationLeaseRepository::new(pool.clone()),
        PostgresContactRevealRepository::new(pool.clone()),
        audit_repo,
        outbox_repo,
        Arc::new(PostgresSellerAccountRepository::new(pool)),
    ))
}

async fn handle_connection<LR, IR, RR, CR>(
    stream: TcpStream,
    app: Arc<MarketplaceApp<LR, IR, RR, CR>>,
    observability: Arc<ServerObservability>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    LR: ListingRepository + Send + Sync,
    IR: IdempotencyKeyRepository + Send + Sync,
    RR: ReservationLeaseRepository + Send + Sync,
    CR: ContactRevealRepository + Send + Sync,
{
    let mut reader = BufReader::new(stream);
    let request = match read_request(&mut reader).await? {
        Some(request) => request,
        None => return Ok(()),
    };

    let response = route_request(&request, app.as_ref()).await;
    observability.record_request(&request.path, response.status);
    write_response(reader.get_mut(), response).await?;
    Ok(())
}

struct HttpRequest {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct HttpResponse {
    status: u16,
    body: Vec<u8>,
    content_type: &'static str,
}

async fn read_request(
    reader: &mut BufReader<TcpStream>,
) -> Result<Option<HttpRequest>, Box<dyn Error + Send + Sync>> {
    let mut start_line = String::new();
    if reader.read_line(&mut start_line).await? == 0 {
        return Ok(None);
    }
    let start_line = start_line.trim_end();
    if start_line.is_empty() {
        return Ok(None);
    }

    let mut parts = start_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();
    let _version = parts.next().unwrap_or_default();

    let (path, query) = split_target(&target);
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value: &String| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await?;
    }

    Ok(Some(HttpRequest {
        method,
        path,
        query,
        headers,
        body,
    }))
}

async fn route_request<LR, IR, RR, CR>(
    request: &HttpRequest,
    app: &MarketplaceApp<LR, IR, RR, CR>,
) -> HttpResponse
where
    LR: ListingRepository + Send + Sync,
    IR: IdempotencyKeyRepository + Send + Sync,
    RR: ReservationLeaseRepository + Send + Sync,
    CR: ContactRevealRepository + Send + Sync,
{
    if request.method == "GET" && request.path == "/health" {
        return json_response(200, serde_json::json!({ "status": "ok" }));
    }

    let claims = match claims_from_headers(&request.headers) {
        Ok(claims) => claims,
        Err(response) => return response,
    };

    match (request.method.as_str(), request.path.as_str()) {
        ("POST", path)
            if path.starts_with("/internal/v1/listings/") && path.ends_with("/archive") =>
        {
            if let Err(response) = authorize_internal_write(&claims) {
                return response;
            }
            let listing_id = path
                .trim_start_matches("/internal/v1/listings/")
                .trim_end_matches("/archive")
                .trim_end_matches('/');
            let body: serde_json::Value = match serde_json::from_slice(&request.body) {
                Ok(value) => value,
                Err(error) => {
                    return api_error_response(
                        400,
                        ApiErrorCode::InvalidField,
                        "invalid archive body",
                        Some(error.to_string()),
                    );
                }
            };
            let Some(reason) = body.get("reason").and_then(|v| v.as_str()) else {
                return api_error_response(
                    400,
                    ApiErrorCode::InvalidField,
                    "archive reason is required",
                    None,
                );
            };
            match app
                .archive_listing(&claims, listing_id, reason, &current_time_marker())
                .await
            {
                Ok(Some(listing)) => json_response(200, serde_json::to_value(listing).unwrap()),
                Ok(None) => {
                    api_error_response(404, ApiErrorCode::NotFound, "listing not found", None)
                }
                Err(error) => map_handler_error(&error),
            }
        }
        ("POST", path)
            if path.starts_with("/internal/v1/listings/")
                && path.ends_with("/release-reservation") =>
        {
            if let Err(response) = authorize_internal_write(&claims) {
                return response;
            }
            let listing_id = path
                .trim_start_matches("/internal/v1/listings/")
                .trim_end_matches("/release-reservation")
                .trim_end_matches('/');
            match serde_json::from_slice::<serde_json::Value>(&request.body) {
                Ok(parsed) => {
                    let Some(reason) = parsed.get("reason").and_then(|value| value.as_str()) else {
                        return api_error_response(
                            400,
                            ApiErrorCode::InvalidField,
                            "internal override reason is required",
                            None,
                        );
                    };
                    let now = current_time_marker();
                    match app
                        .release_reservation(&claims, listing_id, reason, &now)
                        .await
                    {
                        Ok(Some(lease)) => json_response(200, reservation_lease_snapshot(&lease)),
                        Ok(None) => api_error_response(
                            404,
                            ApiErrorCode::NotFound,
                            "reservation not found",
                            None,
                        ),
                        Err(error) => map_handler_error(&error),
                    }
                }
                Err(error) => api_error_response(
                    400,
                    ApiErrorCode::InvalidField,
                    "invalid internal override body",
                    Some(error.to_string()),
                ),
            }
        }
        ("GET", path) if path.starts_with("/internal/v1/listings/") => {
            if let Err(response) = authorize_internal_read(&claims) {
                return response;
            }
            let listing_id = path.trim_start_matches("/internal/v1/listings/");
            match app.get_listing(Some(&claims), listing_id).await {
                Ok(Some(listing)) => json_response(200, serde_json::to_value(listing).unwrap()),
                Ok(None) => {
                    api_error_response(404, ApiErrorCode::NotFound, "listing not found", None)
                }
                Err(error) => map_handler_error(&error),
            }
        }
        ("GET", path) if path.starts_with("/internal/v1/negotiations/") => {
            if let Err(response) = authorize_internal_read(&claims) {
                return response;
            }
            let negotiation_id = path.trim_start_matches("/internal/v1/negotiations/");
            match app.get_negotiation_status(&claims, negotiation_id).await {
                Ok(response) => json_response(200, serde_json::to_value(response).unwrap()),
                Err(error) => map_handler_error(&error),
            }
        }
        ("GET", path) if path.starts_with("/internal/v1/contact-reveals/") => {
            if let Err(response) = authorize_internal_read(&claims) {
                return response;
            }
            let reveal_id = path.trim_start_matches("/internal/v1/contact-reveals/");
            match app.get_contact_reveal(reveal_id).await {
                Ok(Some(response)) => json_response(200, serde_json::to_value(response).unwrap()),
                Ok(None) => api_error_response(
                    404,
                    ApiErrorCode::NotFound,
                    "contact reveal not found",
                    None,
                ),
                Err(error) => map_handler_error(&error),
            }
        }
        ("POST", path)
            if path.starts_with("/internal/v1/sellers/") && path.ends_with("/trust-level") =>
        {
            if let Err(response) = authorize_internal_write(&claims) {
                return response;
            }
            let seller_account_id = path
                .trim_start_matches("/internal/v1/sellers/")
                .trim_end_matches("/trust-level")
                .trim_end_matches('/');
            let body: serde_json::Value = match serde_json::from_slice(&request.body) {
                Ok(value) => value,
                Err(error) => {
                    return api_error_response(
                        400,
                        ApiErrorCode::InvalidField,
                        "invalid trust-level body",
                        Some(error.to_string()),
                    );
                }
            };
            let Some(trust_level) = body.get("trust_level").and_then(|v| v.as_str()) else {
                return api_error_response(
                    400,
                    ApiErrorCode::InvalidField,
                    "trust_level is required",
                    None,
                );
            };
            let Some(reason) = body.get("reason").and_then(|v| v.as_str()) else {
                return api_error_response(
                    400,
                    ApiErrorCode::InvalidField,
                    "reason is required",
                    None,
                );
            };
            match app
                .set_seller_trust_level(
                    &claims,
                    seller_account_id,
                    trust_level,
                    reason,
                    &current_time_marker(),
                )
                .await
            {
                Ok(Some(account)) => json_response(200, serde_json::to_value(account).unwrap()),
                Ok(None) => api_error_response(
                    404,
                    ApiErrorCode::NotFound,
                    "seller account not found",
                    None,
                ),
                Err(error) => map_handler_error(&error),
            }
        }
        ("POST", path)
            if path.starts_with("/internal/v1/sellers/") && path.ends_with("/quota-override") =>
        {
            if let Err(response) = authorize_internal_write(&claims) {
                return response;
            }
            let seller_account_id = path
                .trim_start_matches("/internal/v1/sellers/")
                .trim_end_matches("/quota-override")
                .trim_end_matches('/');
            let body: serde_json::Value = match serde_json::from_slice(&request.body) {
                Ok(value) => value,
                Err(error) => {
                    return api_error_response(
                        400,
                        ApiErrorCode::InvalidField,
                        "invalid quota-override body",
                        Some(error.to_string()),
                    );
                }
            };
            let quota_override = body
                .get("quota_override")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
            let Some(reason) = body.get("reason").and_then(|v| v.as_str()) else {
                return api_error_response(
                    400,
                    ApiErrorCode::InvalidField,
                    "reason is required",
                    None,
                );
            };
            match app
                .set_seller_quota_override(
                    &claims,
                    seller_account_id,
                    quota_override,
                    reason,
                    &current_time_marker(),
                )
                .await
            {
                Ok(Some(account)) => json_response(200, serde_json::to_value(account).unwrap()),
                Ok(None) => api_error_response(
                    404,
                    ApiErrorCode::NotFound,
                    "seller account not found",
                    None,
                ),
                Err(error) => map_handler_error(&error),
            }
        }
        ("GET", "/v1/listings/search") => {
            let search = search_request_from_query(&request.query);
            match app.search_listings(Some(&claims), &search).await {
                Ok(result) => json_response(200, serde_json::to_value(result).unwrap()),
                Err(error) => map_handler_error(&error),
            }
        }
        ("GET", path) if path.starts_with("/v1/listings/") => {
            let listing_id = path.trim_start_matches("/v1/listings/");
            match app.get_listing(Some(&claims), listing_id).await {
                Ok(Some(listing)) => json_response(200, serde_json::to_value(listing).unwrap()),
                Ok(None) => {
                    api_error_response(404, ApiErrorCode::NotFound, "listing not found", None)
                }
                Err(error) => map_handler_error(&error),
            }
        }
        ("POST", "/v1/listings") => {
            match serde_json::from_slice::<CreateListingRequest>(&request.body) {
                Ok(parsed) => {
                    let request_fingerprint = String::from_utf8_lossy(&request.body).to_string();
                    let now = current_time_marker();
                    match app
                        .create_listing(&claims, &parsed, &request_fingerprint, &now)
                        .await
                    {
                        Ok(created) => json_response(201, serde_json::to_value(created).unwrap()),
                        Err(error) => map_handler_error(&error),
                    }
                }
                Err(error) => api_error_response(
                    400,
                    ApiErrorCode::InvalidField,
                    "invalid create listing body",
                    Some(error.to_string()),
                ),
            }
        }
        ("POST", "/v1/negotiations") => {
            match serde_json::from_slice::<OpenNegotiationRequest>(&request.body) {
                Ok(parsed) => {
                    let request_fingerprint = String::from_utf8_lossy(&request.body).to_string();
                    let now = current_time_marker();
                    match app
                        .open_negotiation(&claims, &parsed, &request_fingerprint, &now)
                        .await
                    {
                        Ok(created) => json_response(201, serde_json::to_value(created).unwrap()),
                        Err(error) => map_handler_error(&error),
                    }
                }
                Err(error) => api_error_response(
                    400,
                    ApiErrorCode::InvalidField,
                    "invalid open negotiation body",
                    Some(error.to_string()),
                ),
            }
        }
        ("GET", path)
            if path.starts_with("/v1/negotiations/")
                && !path.ends_with("/request-contact-reveal") =>
        {
            let negotiation_id = path.trim_start_matches("/v1/negotiations/");
            match app.get_negotiation_status(&claims, negotiation_id).await {
                Ok(response) => json_response(200, serde_json::to_value(response).unwrap()),
                Err(error) => map_handler_error(&error),
            }
        }
        ("POST", path) if path.ends_with("/request-contact-reveal") => {
            let negotiation_id = path
                .trim_start_matches("/v1/negotiations/")
                .trim_end_matches("/request-contact-reveal")
                .trim_end_matches('/');
            match serde_json::from_slice::<RequestContactRevealRequest>(&request.body) {
                Ok(parsed) => {
                    let request_fingerprint = String::from_utf8_lossy(&request.body).to_string();
                    let now = current_time_marker();
                    match app
                        .request_contact_reveal(
                            &claims,
                            negotiation_id,
                            &parsed,
                            &request_fingerprint,
                            &now,
                        )
                        .await
                    {
                        Ok(reveal) => json_response(200, serde_json::to_value(reveal).unwrap()),
                        Err(error) => map_handler_error(&error),
                    }
                }
                Err(error) => api_error_response(
                    400,
                    ApiErrorCode::InvalidField,
                    "invalid request-contact-reveal body",
                    Some(error.to_string()),
                ),
            }
        }
        ("POST", path)
            if path.starts_with("/v1/contact-reveals/") && path.ends_with("/approve") =>
        {
            let reveal_id = path
                .trim_start_matches("/v1/contact-reveals/")
                .trim_end_matches("/approve")
                .trim_end_matches('/');
            match app.approve_contact_reveal(&claims, reveal_id).await {
                Ok(reveal) => json_response(200, serde_json::to_value(reveal).unwrap()),
                Err(error) => map_handler_error(&error),
            }
        }
        _ => api_error_response(404, ApiErrorCode::NotFound, "route not found", None),
    }
}

async fn write_response(
    stream: &mut TcpStream,
    response: HttpResponse,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let status_text = match response.status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        status_text,
        response.content_type,
        response.body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(&response.body).await?;
    stream.flush().await?;
    Ok(())
}

fn claims_from_headers(headers: &HashMap<String, String>) -> Result<Claims, HttpResponse> {
    match headers.get("x-marketplace-claims") {
        Some(raw) => serde_json::from_str(raw).map_err(|error| {
            api_error_response(
                400,
                ApiErrorCode::InvalidField,
                "invalid x-marketplace-claims header",
                Some(error.to_string()),
            )
        }),
        None => Err(api_error_response(
            401,
            ApiErrorCode::Unauthorized,
            "missing x-marketplace-claims header",
            None,
        )),
    }
}

fn authorize_internal_read(claims: &Claims) -> Result<(), HttpResponse> {
    if claims.has_role(Role::Admin) || claims.has_role(Role::SupportReviewer) {
        Ok(())
    } else {
        Err(api_error_response(
            403,
            ApiErrorCode::Forbidden,
            "internal route requires admin or support reviewer role",
            None,
        ))
    }
}

fn authorize_internal_write(claims: &Claims) -> Result<(), HttpResponse> {
    if claims.has_role(Role::Admin) {
        Ok(())
    } else {
        Err(api_error_response(
            403,
            ApiErrorCode::Forbidden,
            "internal write route requires admin role",
            None,
        ))
    }
}

fn reservation_lease_snapshot(lease: &crate::models::db::ReservationLeaseRow) -> serde_json::Value {
    serde_json::json!({
        "lease_id": lease.lease_id.clone(),
        "negotiation_id": lease.negotiation_id.clone(),
        "listing_id": lease.listing_id.clone(),
        "reserved_by": lease.reserved_by.clone(),
        "status": lease.status.clone(),
        "expires_at": lease.expires_at.clone(),
        "created_at": lease.created_at.clone(),
        "updated_at": lease.updated_at.clone(),
    })
}

fn search_request_from_query(query: &HashMap<String, String>) -> SearchRequest {
    let price = if query.contains_key("currency")
        || query.contains_key("min_amount")
        || query.contains_key("max_amount")
    {
        Some(SearchPriceFilter {
            currency: query.get("currency").cloned(),
            min_amount: query
                .get("min_amount")
                .and_then(|value: &String| value.parse::<f64>().ok()),
            max_amount: query
                .get("max_amount")
                .and_then(|value: &String| value.parse::<f64>().ok()),
        })
    } else {
        None
    };

    let location = if query.contains_key("country_code") || query.contains_key("city") {
        Some(SearchLocationFilter {
            country_code: query.get("country_code").cloned(),
            city: query.get("city").cloned(),
        })
    } else {
        None
    };

    SearchRequest {
        query: query.get("query").cloned(),
        category: query
            .get("category")
            .and_then(|value| parse_category(value)),
        condition: query
            .get("condition")
            .and_then(|value| parse_condition(value)),
        price,
        location,
        status: query
            .get("status")
            .and_then(|value| parse_listing_status(value)),
        min_seller_rating: query.get("min_seller_rating").and_then(|v| v.parse().ok()),
        verified_sellers_only: query
            .get("verified_sellers_only")
            .and_then(|v| v.parse().ok()),
        sort_by: query
            .get("sort_by")
            .and_then(|value| parse_sort(value))
            .unwrap_or(SearchSort::Relevance),
        limit: query
            .get("limit")
            .and_then(|value: &String| value.parse::<u32>().ok()),
        cursor: query.get("cursor").cloned(),
        // Phase D: Geolocation
        near_me: query.get("near_me").and_then(|v| v.parse().ok()),
        user_latitude: query.get("user_latitude").and_then(|v| v.parse().ok()),
        user_longitude: query.get("user_longitude").and_then(|v| v.parse().ok()),
        radius_km: query.get("radius_km").and_then(|v| v.parse().ok()),
        // NEW: Phase 2 fields
        listing_type: None,
        min_area_sqm: None,
        max_area_sqm: None,

        min_bedrooms: None,

        min_bathrooms: None,

        property_transaction_type: None,
        property_sub_type: None,
        service_type: None,
    }
}

fn parse_category(input: &str) -> Option<Category> {
    match input.to_ascii_lowercase().as_str() {
        "laptop" => Some(Category::Laptop),
        "phone" => Some(Category::Phone),
        "tablet" => Some(Category::Tablet),
        "desktop" => Some(Category::Desktop),
        "monitor" => Some(Category::Monitor),
        "accessory" => Some(Category::Accessory),
        "camera" => Some(Category::Camera),
        "audio" => Some(Category::Audio),
        "gaming" => Some(Category::Gaming),
        "appliance" => Some(Category::Appliance),
        "furniture" => Some(Category::Furniture),
        "vehicle_part" => Some(Category::VehiclePart),
        "other" => Some(Category::Other),
        _ => None,
    }
}

fn parse_condition(input: &str) -> Option<Condition> {
    match input.to_ascii_lowercase().as_str() {
        "new" => Some(Condition::New),
        "used" => Some(Condition::Used),
        "refurbished" => Some(Condition::Refurbished),
        _ => None,
    }
}

fn parse_listing_status(input: &str) -> Option<ListingStatus> {
    match input.to_ascii_lowercase().as_str() {
        "draft" => Some(ListingStatus::Draft),
        "active" => Some(ListingStatus::Active),
        "reserved" => Some(ListingStatus::Reserved),
        "sold" => Some(ListingStatus::Sold),
        "archived" => Some(ListingStatus::Archived),
        _ => None,
    }
}

fn parse_sort(input: &str) -> Option<SearchSort> {
    match input.to_ascii_lowercase().as_str() {
        "relevance" => Some(SearchSort::Relevance),
        "newest" => Some(SearchSort::Newest),
        "price_asc" => Some(SearchSort::PriceAsc),
        "price_desc" => Some(SearchSort::PriceDesc),
        // Phase B: Rating sort
        "rating_highest" => Some(SearchSort::RatingHighest),
        "rating_lowest" => Some(SearchSort::RatingLowest),
        _ => None,
    }
}

fn split_target(target: &str) -> (String, HashMap<String, String>) {
    if let Some((path, query)) = target.split_once('?') {
        (path.to_string(), parse_query_string(query))
    } else {
        (target.to_string(), HashMap::new())
    }
}

fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        map.insert(url_decode(key), url_decode(value));
    }
    map
}

fn url_decode(input: &str) -> String {
    let mut output = String::new();
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &input[index + 1..index + 3];
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    output.push(value as char);
                    index += 3;
                } else {
                    output.push('%');
                    index += 1;
                }
            }
            byte => {
                output.push(byte as char);
                index += 1;
            }
        }
    }
    output
}

fn json_response(status: u16, body: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json",
        body: serde_json::to_vec(&body).unwrap_or_else(|error| {
            serde_json::to_vec(&serde_json::json!({
                "error": {
                    "code": "conflict",
                    "message": "failed to encode response",
                    "field": error.to_string(),
                }
            }))
            .unwrap_or_default()
        }),
    }
}

fn api_error_response(
    status: u16,
    code: ApiErrorCode,
    message: impl Into<String>,
    detail: Option<String>,
) -> HttpResponse {
    json_response(
        status,
        serde_json::json!({
            "error": {
                "code": code,
                "message": message.into(),
                "field": detail,
            }
        }),
    )
}

fn map_handler_error(error: &crate::http::handlers::HandlerError) -> HttpResponse {
    match error {
        crate::http::handlers::HandlerError::Authz(inner) => match inner.kind {
            crate::auth::AuthzErrorKind::MissingScope
            | crate::auth::AuthzErrorKind::MissingRole
            | crate::auth::AuthzErrorKind::OwnershipMismatch => {
                api_error_response(403, ApiErrorCode::Forbidden, inner.message.clone(), None)
            }
        },
        crate::http::handlers::HandlerError::Idempotency(inner) => match inner.kind {
            crate::services::idempotency::IdempotencyErrorKind::InvalidKey => {
                api_error_response(400, ApiErrorCode::InvalidField, inner.message.clone(), None)
            }
            crate::services::idempotency::IdempotencyErrorKind::Conflict => {
                api_error_response(409, ApiErrorCode::Conflict, inner.message.clone(), None)
            }
            crate::services::idempotency::IdempotencyErrorKind::Storage => {
                api_error_response(500, ApiErrorCode::Conflict, inner.message.clone(), None)
            }
        },
        crate::http::handlers::HandlerError::Search(inner) => match inner {
            crate::services::search::SearchError::Authz(authz) => match authz.kind {
                crate::auth::AuthzErrorKind::MissingScope
                | crate::auth::AuthzErrorKind::MissingRole
                | crate::auth::AuthzErrorKind::OwnershipMismatch => {
                    api_error_response(403, ApiErrorCode::Forbidden, authz.message.clone(), None)
                }
            },
            crate::services::search::SearchError::Storage(storage) => {
                api_error_response(500, ApiErrorCode::Conflict, storage.to_string(), None)
            }
        },
        crate::http::handlers::HandlerError::Repository(repository) => match repository.kind {
            crate::repositories::RepositoryErrorKind::Conflict => api_error_response(
                409,
                ApiErrorCode::Conflict,
                repository.message.clone(),
                None,
            ),
            crate::repositories::RepositoryErrorKind::NotFound => api_error_response(
                404,
                ApiErrorCode::NotFound,
                repository.message.clone(),
                None,
            ),
            crate::repositories::RepositoryErrorKind::PermissionDenied => api_error_response(
                403,
                ApiErrorCode::Forbidden,
                repository.message.clone(),
                None,
            ),
            crate::repositories::RepositoryErrorKind::Validation => api_error_response(
                400,
                ApiErrorCode::InvalidField,
                repository.message.clone(),
                None,
            ),
            crate::repositories::RepositoryErrorKind::Storage
            | crate::repositories::RepositoryErrorKind::Unknown => api_error_response(
                500,
                ApiErrorCode::Conflict,
                repository.message.clone(),
                None,
            ),
        },
        crate::http::handlers::HandlerError::QuotaExceeded { message } => {
            api_error_response(403, ApiErrorCode::Forbidden, message.clone(), None)
        }
    }
}

pub fn current_time_marker() -> String {
    // Use chrono::Utc::now() to get current time in RFC3339 format
    let now = chrono::Utc::now();
    now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::audit_events::InMemoryAuditEventRepository;
    use crate::repositories::listings::InMemoryListingRepository;
    use crate::repositories::outbox_events::InMemoryOutboxEventRepository;
    use marketplace_api_contract::{
        Category, Condition, CreateListingRequest, ListingLocation, ListingPayload, Price,
    };
    use marketplace_auth_core::{Claims, Role};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn claims() -> Claims {
        crate::test_support::seller_claims()
    }

    fn admin_claims() -> Claims {
        crate::test_support::admin_claims()
    }

    fn reviewer_claims() -> Claims {
        crate::test_support::support_claims()
    }

    fn claims_header_for(claims: &Claims) -> String {
        serde_json::to_string(claims).unwrap()
    }

    fn claims_header() -> String {
        claims_header_for(&claims())
    }

    fn http_request(
        method: &str,
        path: &str,
        claims: Option<&Claims>,
        body: Option<&str>,
    ) -> String {
        let claims_header = claims.map(claims_header_for).unwrap_or_default();
        let body = body.unwrap_or("");
        let content_length = body.len();
        if claims.is_some() {
            format!(
                "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nX-Marketplace-Claims: {claims_header}\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n{body}"
            )
        } else if content_length > 0 {
            format!(
                "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n{body}"
            )
        } else {
            format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        }
    }

    fn json_request(
        method: &str,
        path: &str,
        claims: Option<&Claims>,
        body: serde_json::Value,
    ) -> String {
        let body = body.to_string();
        http_request(method, path, claims, Some(&body))
    }

    fn create_body() -> String {
        serde_json::to_string(&CreateListingRequest {
            idempotency_key: "idem-create-1".to_string(),
            listing: ListingPayload {
                schema_version: "1.0".to_string(),
                owner_id: "seller-1".to_string(),
                listing_type: marketplace_api_contract::ListingType::Product,
                category: Some(Category::Laptop),
                title: "ThinkPad T480".to_string(),
                condition: Some(Condition::Used),
                price: Price {
                    currency: "USD".to_string(),
                    amount: 450.0,
                },
                location: ListingLocation {
                    country_code: "JP".to_string(),
                    country_name: "Japan".to_string(),
                    city: "Osaka".to_string(),
                    // Phase D: Geolocation (optional)
                    latitude: None,
                    longitude: None,
                    geolocation_opt_out: None,
                },
                picture_urls: vec!["https://example.com/item.jpg".to_string()],
                description: "Good battery health".to_string(),
                attributes: None,
                // Marketplace fields
                sku: None,
                quantity: None,
                shipping_info: None,
                condition_details: None,
                seller_notes: None,
                // Phase 4: Service fields (None for Product)
                service_type: None,
                hourly_rate: None,
                project_rate: None,
                qualifications: None,
                service_radius_km: None,
                // Phase 4: Property fields (None for Product)
                property_transaction_type: None,
                property_sub_type: None,
                area_sqm: None,
                bedrooms: None,
                bathrooms: None,
                year_built: None,
                lot_size_sqm: None,
                zoning: None,
            },
        })
        .unwrap()
    }

    async fn round_trip(address: &str, request: &str) -> String {
        let mut stream = TcpStream::connect(address).await.unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        stream.shutdown().await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        String::from_utf8(buf).unwrap()
    }

    type TestRuntimeApp = MarketplaceApp<
        InMemoryListingRepository,
        InMemoryIdempotencyRepository,
        crate::repositories::reservations::InMemoryReservationLeaseRepository,
        crate::repositories::contact_reveals::InMemoryContactRevealRepository,
    >;

    fn build_runtime_app_for_test() -> Arc<TestRuntimeApp> {
        Arc::new(MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            crate::repositories::reservations::InMemoryReservationLeaseRepository::new(),
            crate::repositories::contact_reveals::InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(crate::repositories::seller_accounts::InMemorySellerAccountRepository::new()),
        ))
    }

    #[tokio::test]
    async fn runtime_hits_health_and_listings() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let app = build_runtime_app_for_test();
        let accept_app = Arc::clone(&app);
        tokio::spawn(async move {
            for _ in 0..7 {
                let (stream, _) = listener.accept().await.unwrap();
                let app = Arc::clone(&accept_app);
                let observability = Arc::new(ServerObservability::new());
                tokio::spawn(async move {
                    let _ = handle_connection(stream, app, observability).await;
                });
            }
        });

        let health = round_trip(
            &address,
            "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(health.contains("\"status\":\"ok\""));

        let create = round_trip(
            &address,
            &format!(
                "POST /v1/listings HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nX-Marketplace-Claims: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                claims_header(),
                create_body().len(),
                create_body()
            ),
        )
        .await;
        assert!(create.contains("ThinkPad T480"));

        let get = round_trip(
            &address,
            &format!(
                "GET /v1/listings/lst_000001 HTTP/1.1\r\nHost: localhost\r\nX-Marketplace-Claims: {}\r\nConnection: close\r\n\r\n",
                claims_header()
            ),
        )
        .await;
        assert!(get.contains("ThinkPad T480"));
        assert!(get.contains("\"listing_id\":\"lst_000001\""));

        let open = round_trip(
            &address,
            &format!(
                "POST /v1/negotiations HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nX-Marketplace-Claims: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                claims_header(),
                serde_json::json!({
                    "idempotency_key": "idem-open-1",
                    "listing_id": "lst_000001",
                    "buyer_agent_id": "buyer-1",
                    "offer_currency": "USD",
                    "offer_amount": 440.0
                })
                .to_string()
                .len(),
                serde_json::json!({
                    "idempotency_key": "idem-open-1",
                    "listing_id": "lst_000001",
                    "buyer_agent_id": "buyer-1",
                    "offer_currency": "USD",
                    "offer_amount": 440.0
                })
            ),
        )
        .await;
        assert!(open.contains("\"status\":\"reserved\""));

        let internal_listing = round_trip(
            &address,
            &format!(
                "GET /internal/v1/listings/lst_000001 HTTP/1.1\r\nHost: localhost\r\nX-Marketplace-Claims: {}\r\nConnection: close\r\n\r\n",
                serde_json::to_string(&reviewer_claims()).unwrap()
            ),
        )
        .await;
        assert!(internal_listing.contains("\"listing_id\":\"lst_000001\""));

        let denied = round_trip(
            &address,
            &format!(
                "GET /internal/v1/listings/lst_000001 HTTP/1.1\r\nHost: localhost\r\nX-Marketplace-Claims: {}\r\nConnection: close\r\n\r\n",
                claims_header()
            ),
        )
        .await;
        assert!(denied.contains("\"code\":\"forbidden\""));

        let release_body = serde_json::json!({ "reason": "admin cleanup" }).to_string();
        let release = round_trip(
            &address,
            &format!(
                "POST /internal/v1/listings/lst_000001/release-reservation HTTP/1.1\r\nHost: localhost\r\nX-Marketplace-Claims: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                serde_json::to_string(&admin_claims()).unwrap(),
                release_body.len(),
                release_body
            ),
        )
        .await;
        assert!(release.contains("\"status\":\"cancelled\""));
    }

    #[tokio::test]
    async fn runtime_returns_unauthorized_without_claims_header() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let app = build_runtime_app_for_test();
        let accept_app = Arc::clone(&app);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let observability = Arc::new(ServerObservability::new());
            let _ = handle_connection(stream, accept_app, observability).await;
        });

        let response = round_trip(
            &address,
            "GET /v1/listings/search HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await;

        assert!(response.contains("\"code\":\"unauthorized\""));
        assert!(response.contains("\"missing x-marketplace-claims header\""));
    }

    #[tokio::test]
    async fn runtime_returns_forbidden_for_internal_route_without_admin_role() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let app = build_runtime_app_for_test();
        let accept_app = Arc::clone(&app);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let observability = Arc::new(ServerObservability::new());
            let _ = handle_connection(stream, accept_app, observability).await;
        });

        let response = round_trip(
            &address,
            &format!(
                "GET /internal/v1/listings/lst_000001 HTTP/1.1\r\nHost: localhost\r\nX-Marketplace-Claims: {}\r\nConnection: close\r\n\r\n",
                claims_header()
            ),
        )
        .await;

        assert!(response.contains("\"code\":\"forbidden\""));
        assert!(response.contains("internal route requires admin or support reviewer role"));
    }

    #[tokio::test]
    async fn runtime_returns_forbidden_for_internal_write_without_admin_role() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let app = build_runtime_app_for_test();
        let accept_app = Arc::clone(&app);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let observability = Arc::new(ServerObservability::new());
            let _ = handle_connection(stream, accept_app, observability).await;
        });

        let response = round_trip(
            &address,
            &json_request(
                "POST",
                "/internal/v1/listings/lst_000001/archive",
                Some(&reviewer_claims()),
                serde_json::json!({ "reason": "cleanup" }),
            ),
        )
        .await;

        assert!(response.contains("\"code\":\"forbidden\""));
        assert!(response.contains("internal write route requires admin role"));
    }

    #[tokio::test]
    async fn runtime_returns_forbidden_for_create_listing_without_role() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let app = build_runtime_app_for_test();
        let accept_app = Arc::clone(&app);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let observability = Arc::new(ServerObservability::new());
            let _ = handle_connection(stream, accept_app, observability).await;
        });

        let mut denied_claims = claims();
        denied_claims.roles = vec![Role::BuyerNegotiator];
        let response = round_trip(
            &address,
            &http_request(
                "POST",
                "/v1/listings",
                Some(&denied_claims),
                Some(&create_body()),
            ),
        )
        .await;

        assert!(response.contains("\"code\":\"forbidden\""));
        assert!(response.contains("missing required role"));
    }

    #[tokio::test]
    async fn runtime_allows_admin_internal_archive() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let app = build_runtime_app_for_test();
        let accept_app = Arc::clone(&app);
        tokio::spawn(async move {
            for _ in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let observability = Arc::new(ServerObservability::new());
                let app = Arc::clone(&accept_app);
                tokio::spawn(async move {
                    let _ = handle_connection(stream, app, observability).await;
                });
            }
        });

        let created = round_trip(
            &address,
            &json_request(
                "POST",
                "/v1/listings",
                Some(&claims()),
                serde_json::from_str::<serde_json::Value>(&create_body()).unwrap(),
            ),
        )
        .await;
        assert!(created.contains("\"listing_id\":\"lst_000001\""));

        let archived = round_trip(
            &address,
            &json_request(
                "POST",
                "/internal/v1/listings/lst_000001/archive",
                Some(&admin_claims()),
                serde_json::json!({ "reason": "cleanup" }),
            ),
        )
        .await;
        assert!(archived.contains("\"status\":\"archived\""));
    }
}
