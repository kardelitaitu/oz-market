//! OpenAPI documentation generator
//! 
//! Uses utoipa to auto-generate OpenAPI spec from code annotations.
//! The spec can be generated as JSON/YAML and served or saved.

use utoipa::OpenApi;

/// Main API documentation struct
/// 
/// Aggregates all paths and schemas from annotated handlers.
#[derive(OpenApi)]
#[openapi(
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

/// Actix handler to serve OpenAPI JSON
pub async fn serve_openapi_json() -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok()
        .content_type("application/json")
        .body(generate_openapi_json())
}
