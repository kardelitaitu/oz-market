//! OpenAPI documentation generator
//! 
//! Manually constructs OpenAPI spec (avoids utoipa macro issues).
//! The spec can be served via the /api-docs/openapi.json endpoint.

use serde_json::{json, Value};

/// Generate OpenAPI JSON spec manually
pub fn generate_openapi_json() -> String {
    let spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Marketplace API",
            "version": "1.0.0",
            "description": "Decentralized marketplace API with AI prompt caching"
        },
        "servers": [
            {
                "url": "http://localhost:3003",
                "description": "Local development server"
            }
        ],
        "tags": [
            {"name": "listings", "description": "Listing management"},
            {"name": "search", "description": "Search endpoints"},
            {"name": "reviews", "description": "Review endpoints"},
            {"name": "admin", "description": "Admin endpoints"}
        ],
        "paths": {
            "/v1/listings/search": {
                "get": {
                    "tags": ["search"],
                    "summary": "Search listings",
                    "parameters": [
                        {
                            "name": "query",
                            "in": "query",
                            "description": "Search query text",
                            "schema": {"type": "string"}
                        },
                        {
                            "name": "category",
                            "in": "query",
                            "description": "Filter by category",
                            "schema": {"type": "string"}
                        },
                        {
                            "name": "min_seller_rating",
                            "in": "query",
                            "description": "Minimum seller rating (1-5)",
                            "schema": {"type": "number"}
                        },
                        {
                            "name": "sort_by",
                            "in": "query", 
                            "description": "Sort order",
                            "schema": {"type": "string", "enum": ["relevance", "newest", "price_asc", "price_desc", "rating_highest", "rating_lowest"]}
                        },
                        {
                            "name": "limit",
                            "in": "query",
                            "description": "Max results",
                            "schema": {"type": "integer"}
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Search results",
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/SearchResponse"}
                                }
                            }
                        },
                        "400": {"description": "Invalid search parameters"},
                        "401": {"description": "Unauthorized"}
                    }
                }
            },
            "/v1/listings/{listing_id}": {
                "get": {
                    "tags": ["listings"],
                    "summary": "Get a listing by ID",
                    "parameters": [
                        {
                            "name": "listing_id",
                            "in": "path",
                            "required": true,
                            "description": "Listing ID",
                            "schema": {"type": "string"}
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Listing found",
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/ListingSummary"}
                                }
                            }
                        },
                        "401": {"description": "Unauthorized"},
                        "404": {"description": "Listing not found"}
                    }
                }
            },
            "/v1/listings": {
                "post": {
                    "tags": ["listings"],
                    "summary": "Create a new listing",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/CreateListingRequest"}
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Listing created",
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/CreateListingResponse"}
                                }
                            }
                        },
                        "400": {"description": "Invalid request"},
                        "401": {"description": "Unauthorized"},
                        "409": {"description": "Idempotency conflict"}
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "SearchResponse": {
                    "type": "object",
                    "properties": {
                        "items": {"type": "array", "items": {"$ref": "#/components/schemas/ListingSummary"}},
                        "applied_sort_by": {"$ref": "#/components/schemas/SearchSort"},
                        "next_cursor": {"type": "string"}
                    }
                },
                "ListingSummary": {
                    "type": "object",
                    "properties": {
                        "listing_id": {"type": "string"},
                        "status": {"type": "string"},
                        "listing": {"$ref": "#/components/schemas/ListingPayload"}
                    }
                },
                "ListingPayload": {
                    "type": "object",
                    "properties": {
                        "product_name": {"type": "string"},
                        "price": {"$ref": "#/components/schemas/Price"},
                        "category": {"type": "string"}
                    }
                },
                "Price": {
                    "type": "object",
                    "properties": {
                        "currency": {"type": "string"},
                        "amount": {"type": "number"}
                    }
                },
                "SearchSort": {
                    "type": "string",
                    "enum": ["relevance", "newest", "price_asc", "price_desc", "rating_highest", "rating_lowest"]
                }
            }
        }
    });
    
    serde_json::to_string_pretty(&spec).unwrap_or_else(|e| {
        eprintln!("Failed to serialize OpenAPI spec: {}", e);
        "{}".to_string()
    })
}

/// Generate OpenAPI YAML spec (converts JSON to YAML)
pub fn generate_openapi_yaml() -> String {
    let json_spec = generate_openapi_json();
    // For simplicity, just return JSON for now (YAML requires additional dependency)
    json_spec
}

/// Actix handler to serve OpenAPI JSON
pub async fn serve_openapi_json() -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok()
        .content_type("application/json")
        .body(generate_openapi_json())
}
