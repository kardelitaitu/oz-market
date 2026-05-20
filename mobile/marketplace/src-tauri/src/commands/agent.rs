use marketplace_api_contract::{AgentQueryRequest, AgentQueryResponse};
use serde::Deserialize;
use tauri::State;

use crate::auth;
use crate::client::ApiClient;
use crate::state::AppState;

async fn build_client(state: &AppState) -> Result<ApiClient, String> {
    let base_url = state.base_url.read().await.clone();
    Ok(ApiClient::new(
        state.client.clone(),
        base_url,
        state.rate_limiter.clone(),
    ))
}

#[derive(Deserialize)]
pub struct AgentQueryParams {
    pub query: String,
    pub conversation_id: Option<String>,
}

#[tauri::command]
pub async fn agent_query(
    state: State<'_, AppState>,
    params: AgentQueryParams,
) -> Result<AgentQueryResponse, String> {
    let claims = auth::load_claims()?;
    let client = build_client(&state).await?;

    let request = AgentQueryRequest {
        query: params.query,
        conversation_id: params.conversation_id,
    };

    client
        .agent_query(&claims, &request)
        .await
        .map_err(|e| e.to_string())
}
