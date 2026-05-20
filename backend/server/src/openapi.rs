//! OpenAPI documentation generator
//!
//! Serves the existing OpenAPI spec from docs/specs/openapi.yaml.
//! Provides a simple redirect to Swagger Editor for interactive docs.

use serde_json;
use serde_yaml;
use std::fs;
use std::path::PathBuf;
use actix_web::HttpRequest;

/// Try to locate the openapi.yaml file
fn find_openapi_yaml() -> Option<PathBuf> {
    let candidates = vec![
        PathBuf::from("docs/specs/openapi.yaml"),
        PathBuf::from("../../docs/specs/openapi.yaml"), // from backend/server/
    ];
    candidates.into_iter().find(|p| p.exists())
}

/// Generate OpenAPI JSON spec by reading the YAML file and converting to JSON
pub fn generate_openapi_json() -> String {
    if let Some(path) = find_openapi_yaml() {
        match fs::read_to_string(&path) {
            Ok(yaml_str) => match serde_yaml::from_str::<serde_yaml::Value>(&yaml_str) {
                Ok(yaml_value) => match serde_json::to_string_pretty(&yaml_value) {
                    Ok(json_str) => return json_str,
                    Err(e) => {
                        eprintln!("Failed to convert YAML to JSON: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to parse YAML: {}", e);
                }
            },
            Err(e) => {
                eprintln!("Failed to read {}: {}", path.display(), e);
            }
        }
    } else {
        eprintln!("Could not find openapi.yaml");
    }
    // Fallback: return minimal spec
    "{}".to_string()
}

/// Generate OpenAPI YAML spec (just return the YAML content)
pub fn generate_openapi_yaml() -> String {
    if let Some(path) = find_openapi_yaml() {
        match fs::read_to_string(&path) {
            Ok(yaml_str) => return yaml_str,
            Err(e) => {
                eprintln!("Failed to read {}: {}", path.display(), e);
            }
        }
    }
    "".to_string()
}

/// Actix handler to serve OpenAPI JSON
pub async fn serve_openapi_json() -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok()
        .content_type("application/json")
        .body(generate_openapi_json())
}

/// Serve a simple HTML page that redirects to Swagger Editor with our spec
pub async fn serve_swagger_editor(req: HttpRequest) -> impl actix_web::Responder {
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost:3000");
    let spec_url = format!("http://{host}/api-docs/openapi.json");
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Marketplace API Docs</title>
    <meta http-equiv="refresh" content="0; url=https://editor.swagger.io/?url={spec_url}">
</head>
<body>
    <p>Redirecting to Swagger Editor... <a href="https://editor.swagger.io/?url={spec_url}">click here</a> if not redirected.</p>
</body>
</html>"#
    );
    actix_web::HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}
