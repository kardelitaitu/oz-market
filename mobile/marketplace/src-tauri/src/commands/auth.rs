use crate::auth;
use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::State;

#[tauri::command]
pub async fn login(
    sub: String,
    seller_account_id: Option<String>,
    buyer_agent_id: Option<String>,
    roles: Vec<String>,
    scopes: Vec<String>,
) -> Result<String, String> {
    let claims = auth::Claims {
        sub,
        roles,
        scopes,
        seller_account_id,
        buyer_agent_id,
        hardware_id: None,
        exp: None,
    };
    auth::store_claims(&claims)?;
    let json = serde_json::to_string(&claims).map_err(|e| e.to_string())?;
    Ok(json)
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let mut listeners = state.negotiation_listeners.write().await;
    for (_, cancelled) in listeners.drain() {
        cancelled.store(true, Ordering::Relaxed);
    }
    auth::clear_claims()
}

#[tauri::command]
pub async fn get_claims() -> Result<String, String> {
    let claims = auth::load_claims()?;
    serde_json::to_string(&claims).map_err(|e| e.to_string())
}
