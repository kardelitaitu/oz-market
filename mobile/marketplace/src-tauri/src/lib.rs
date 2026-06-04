mod auth;
mod client;
mod commands;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::auth::login,
            commands::auth::logout,
            commands::auth::get_claims,
            commands::listings::health,
            commands::listings::get_listing,
            commands::listings::search_listings,
            commands::listings::set_base_url,
            commands::listings::get_base_url,
            commands::listings::create_listing,
            commands::listings::my_listings,
            commands::negotiations::open_negotiation,
            commands::negotiations::get_negotiation,
            commands::negotiations::submit_offer,
            commands::negotiations::accept_negotiation,
            commands::negotiations::reject_negotiation,
            commands::negotiations::request_contact_reveal,
            commands::negotiations::approve_contact_reveal,
            commands::agent::agent_query,
            commands::notifications::request_notification_permission,
            commands::notifications::send_local_notification,
            commands::rate_limits::get_rate_limits,
            commands::negotiations::start_negotiation_listener,
            commands::negotiations::stop_negotiation_listener,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
