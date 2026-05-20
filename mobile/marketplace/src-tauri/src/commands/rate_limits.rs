use crate::client::rate_limit::RateLimitSummary;
use crate::state::AppState;
use tauri::State;

/// Return all currently tracked rate limits from the client-side tracker.
/// The Svelte UI polls this to display remaining quota indicators.
#[tauri::command]
pub async fn get_rate_limits(state: State<'_, AppState>) -> Result<Vec<RateLimitSummary>, String> {
    let tracker = state.rate_limiter.read().await;
    Ok(tracker.all_limits())
}
