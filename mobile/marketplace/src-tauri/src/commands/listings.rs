use marketplace_api_contract::{
    CreateListingRequest, ListingLocation, ListingPayload, ListingSummary, ListingType,
    Price as ListingPrice, SearchRequest, SearchResponse,
};
use serde::Deserialize;
use serde_json::Value;
use tauri::State;

use crate::auth;
use crate::client::ApiClient;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateListingParams {
    pub title: String,
    pub description: String,
    pub listing_type: String,
    pub currency: String,
    pub amount: f64,
    pub country_code: String,
    pub city: String,
    pub idempotency_key: String,
}

async fn build_client(state: &AppState) -> Result<ApiClient, String> {
    let base_url = state.base_url.read().await.clone();
    Ok(ApiClient::new(
        state.client.clone(),
        base_url,
        state.rate_limiter.clone(),
    ))
}

#[tauri::command]
pub async fn health(state: State<'_, AppState>) -> Result<Value, String> {
    let client = build_client(&state).await?;
    client.health().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_listing(
    state: State<'_, AppState>,
    listing_id: String,
) -> Result<ListingSummary, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;
    client
        .get_listing(&claims, &listing_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_listings(
    state: State<'_, AppState>,
    query: Option<String>,
    _category: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Result<SearchResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let request = SearchRequest {
        query,
        limit,
        cursor,
        ..Default::default()
    };

    client
        .search_listings(&claims, &request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_base_url(state: State<'_, AppState>, url: String) -> Result<(), String> {
    let mut base_url = state.base_url.write().await;
    *base_url = url;
    Ok(())
}

#[tauri::command]
pub async fn get_base_url(state: State<'_, AppState>) -> Result<String, String> {
    let base_url = state.base_url.read().await;
    Ok(base_url.clone())
}

#[tauri::command]
pub async fn create_listing(
    state: State<'_, AppState>,
    params: CreateListingParams,
) -> Result<ListingSummary, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let listing_type_enum = match params.listing_type.as_str() {
        "product" => ListingType::Product,
        "service" => ListingType::Service,
        "property" => ListingType::Property,
        _ => return Err(format!("Invalid listing type: {}", params.listing_type)),
    };

    let request = CreateListingRequest {
        idempotency_key: params.idempotency_key,
        listing: ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: claims.sub.clone(),
            listing_type: listing_type_enum,
            title: params.title,
            description: params.description,
            price: ListingPrice {
                currency: params.currency,
                amount: params.amount,
            },
            location: ListingLocation {
                country_code: params.country_code,
                country_name: String::new(),
                city: params.city,
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            category: None,
            condition: None,
            picture_urls: Vec::new(),
            attributes: None,
            sku: None,
            quantity: None,
            shipping_info: None,
            condition_details: None,
            seller_notes: None,
            service_type: None,
            hourly_rate: None,
            project_rate: None,
            qualifications: None,
            service_radius_km: None,
            property_transaction_type: None,
            property_sub_type: None,
            area_sqm: None,
            bedrooms: None,
            bathrooms: None,
            year_built: None,
            lot_size_sqm: None,
            zoning: None,
        },
    };

    client
        .create_listing(&claims, &request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn my_listings(
    state: State<'_, AppState>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Result<SearchResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let request = SearchRequest {
        owner_id: Some(claims.sub.clone()),
        limit,
        cursor,
        ..Default::default()
    };

    client
        .search_listings(&claims, &request)
        .await
        .map_err(|e| e.to_string())
}
