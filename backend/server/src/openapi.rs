//! OpenAPI documentation generator
//! 
//! Uses utoipa to auto-generate OpenAPI spec from code annotations.
//! The spec can be generated as JSON/YAML and served or saved.

use utoipa::OpenApi;
use crate::http::actix_handlers::{search_listings, get_listing, create_listing};
use marketplace_api_contract::{
    SearchRequest, SearchResponse, CreateListingRequest, CreateListingResponse,
    ListingSummary, Category, SearchSort, ListingStatus, ListingPayload,
    Price, ListingLocation, ShippingInfo, SearchPriceFilter, SearchLocationFilter,
};

/// Main API documentation struct
/// 
/// Aggregates all paths and schemas from annotated handlers.
#[derive(OpenApi)]
#[openapi(
    paths(
        search_listings,
        get_listing,
        create_listing
    ),
    components(
        schemas(
            SearchRequest,
            SearchResponse,
            CreateListingRequest,
            CreateListingResponse,
            ListingSummary,
            Category,
            SearchSort,
            ListingStatus,
            ListingPayload,
            Price,
            ListingLocation,
            ShippingInfo,
            SearchPriceFilter,
            SearchLocationFilter
        )
    ),
    tags(
        (name = "listings", description = "Listing management"),
        (name = "search", description = "Search endpoints"),
        (name = "health", description = "Health checks")
    ),
    info(
        title = "Marketplace API",
        version = "1.0.0",
        description = "Decentralized marketplace API with AI prompt caching"
    )
)]
pub struct ApiDoc;

/// Generate OpenAPI JSON spec (for serving or saving)
pub fn generate_openapi_json() -> String {
    match ApiDoc::openapi().to_json() {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Failed to generate OpenAPI JSON: {}", e);
            "{}".to_string()
        }
    }
}

/// Generate OpenAPI YAML spec
pub fn generate_openapi_yaml() -> String {
    match ApiDoc::openapi().to_yaml() {
        Ok(yaml) => yaml,
        Err(e) => {
            eprintln!("Failed to generate OpenAPI YAML: {}", e);
            "".to_string()
        }
    }
}
