//! OpenAPI documentation generator
//! 
//! Serves the existing OpenAPI spec from docs/specs/openapi.yaml.
//! Converts YAML to JSON for the /api-docs/openapi.json endpoint.

use serde_json;
use serde_yaml;
use std::fs;
use std::path::PathBuf;

/// Try to locate the openapi.yaml file
fn find_openapi_yaml() -> Option<PathBuf> {
    let candidates = vec![
        PathBuf::from("docs/specs/openapi.yaml"),
        PathBuf::from("../../docs/specs/openapi.yaml"), // from backend/server/
        PathBuf::from("C:/My Script/project-the-marketplace/docs/specs/openapi.yaml"),
    ];
    for path in candidates {
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Generate OpenAPI JSON spec by reading the YAML file and converting to JSON
pub fn generate_openapi_json() -> String {
    if let Some(path) = find_openapi_yaml() {
        match fs::read_to_string(&path) {
            Ok(yaml_str) => {
                match serde_yaml::from_str::<serde_yaml::Value>(&yaml_str) {
                    Ok(yaml_value) => {
                        match serde_json::to_string_pretty(&yaml_value) {
                            Ok(json_str) => return json_str,
                            Err(e) => {
                                eprintln!("Failed to convert YAML to JSON: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse YAML: {}", e);
                    }
                }
            }
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
