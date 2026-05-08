//! OpenAPI documentation generator
//! 
//! Uses utoipa to auto-generate OpenAPI spec from code annotations.
//! The spec can be served via Swagger UI or exported as JSON/YAML.

use utoipa::OpenApi;

/// Main API documentation struct
/// 
/// This struct will aggregate all paths and schemas when we annotate handlers.
#[derive(OpenApi)]
#[openapi(
    paths(),
    components(
        schemas()
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "listings", description = "Listing management endpoints"),
        (name = "search", description = "Search endpoints"),
        (name = "reviews", description = "Review endpoints"),
        (name = "admin", description = "Admin endpoints")
    ),
    info(
        title = "Marketplace API",
        version = "1.0.0",
        description = "API for the decentralized marketplace",
        contact(
            name = "Marketplace Team",
            email = "contact@marketplace.example"
        )
    )
)]
pub struct ApiDoc;

/// Generate OpenAPI JSON spec
pub fn generate_openapi_json() -> String {
    ApiDoc::openapi().to_json().unwrap_or_else(|e| {
        eprintln!("Failed to generate OpenAPI JSON: {}", e);
        "{}".to_string()
    })
}

/// Generate OpenAPI YAML spec  
pub fn generate_openapi_yaml() -> String {
    ApiDoc::openapi().to_yaml().unwrap_or_else(|e| {
        eprintln!("Failed to generate OpenAPI YAML: {}", e);
        "".to_string()
    })
}
