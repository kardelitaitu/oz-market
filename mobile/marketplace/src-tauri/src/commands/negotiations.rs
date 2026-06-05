use oz_market_api_contract::{
    AcceptNegotiationRequest, ContactRevealResponse, NegotiationResponse, OpenNegotiationRequest,
    RejectNegotiationRequest, RequestContactRevealRequest, SubmitOfferRequest,
};
use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::auth;
use crate::client::{sse, ApiClient};
use crate::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

async fn build_client(state: &AppState) -> Result<ApiClient, String> {
    let base_url = state.base_url.read().await.clone();
    Ok(ApiClient::new(
        state.client.clone(),
        base_url,
        state.rate_limiter.clone(),
    ))
}

#[derive(Deserialize)]
pub struct OpenNegotiationParams {
    pub listing_id: String,
    pub currency: String,
    pub amount: f64,
    pub idempotency_key: String,
}

#[derive(Deserialize)]
pub struct OfferParams {
    pub negotiation_id: String,
    pub currency: String,
    pub amount: f64,
    pub idempotency_key: String,
}

#[derive(Deserialize)]
pub struct NegotiationIdParams {
    pub negotiation_id: String,
}

#[derive(Deserialize)]
pub struct IdempotentParams {
    pub idempotency_key: String,
}

#[tauri::command]
pub async fn open_negotiation(
    state: State<'_, AppState>,
    params: OpenNegotiationParams,
) -> Result<NegotiationResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let request = OpenNegotiationRequest {
        listing_id: params.listing_id,
        buyer_agent_id: claims.sub.clone(),
        offer_currency: params.currency,
        offer_amount: params.amount,
        idempotency_key: params.idempotency_key,
    };

    client
        .open_negotiation(&claims, &request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_negotiation(
    state: State<'_, AppState>,
    params: NegotiationIdParams,
) -> Result<NegotiationResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    client
        .get_negotiation(&claims, &params.negotiation_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn submit_offer(
    state: State<'_, AppState>,
    params: OfferParams,
) -> Result<NegotiationResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let request = SubmitOfferRequest {
        offer_currency: params.currency,
        offer_amount: params.amount,
        idempotency_key: params.idempotency_key,
    };

    client
        .submit_offer(&claims, &params.negotiation_id, &request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn accept_negotiation(
    state: State<'_, AppState>,
    negotiation_id: String,
    params: IdempotentParams,
) -> Result<NegotiationResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let request = AcceptNegotiationRequest {
        idempotency_key: params.idempotency_key,
    };

    client
        .accept_negotiation(&claims, &negotiation_id, &request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reject_negotiation(
    state: State<'_, AppState>,
    negotiation_id: String,
    params: IdempotentParams,
) -> Result<NegotiationResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let request = RejectNegotiationRequest {
        idempotency_key: params.idempotency_key,
    };

    client
        .reject_negotiation(&claims, &negotiation_id, &request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn request_contact_reveal(
    state: State<'_, AppState>,
    negotiation_id: String,
    params: IdempotentParams,
) -> Result<ContactRevealResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let request = RequestContactRevealRequest {
        idempotency_key: params.idempotency_key,
    };

    client
        .request_contact_reveal(&claims, &negotiation_id, &request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn approve_contact_reveal(
    state: State<'_, AppState>,
    reveal_id: String,
    params: IdempotentParams,
) -> Result<ContactRevealResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let request = RequestContactRevealRequest {
        idempotency_key: params.idempotency_key,
    };

    client
        .approve_contact_reveal(&claims, &reveal_id, &request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_negotiation_listener(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    negotiation_id: String,
) -> Result<(), String> {
    let cancelled = Arc::new(AtomicBool::new(false));

    {
        let mut listeners = state.negotiation_listeners.write().await;
        if let Some(old) = listeners.remove(&negotiation_id) {
            old.store(true, Ordering::Relaxed);
        }
        listeners.insert(negotiation_id.clone(), cancelled.clone());
    }

    let inner = AppState {
        client: state.client.clone(),
        base_url: state.base_url.clone(),
        rate_limiter: state.rate_limiter.clone(),
        negotiation_listeners: state.negotiation_listeners.clone(),
    };
    tokio::spawn(async move {
        sse::listen_negotiation(app_handle, inner, negotiation_id, cancelled).await;
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_negotiation_listener(
    state: State<'_, AppState>,
    negotiation_id: String,
) -> Result<(), String> {
    let mut listeners = state.negotiation_listeners.write().await;
    if let Some(cancelled) = listeners.remove(&negotiation_id) {
        cancelled.store(true, Ordering::Relaxed);
    }
    Ok(())
}
