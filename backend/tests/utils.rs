use std::env;
use std::process::Command;

/// Start a temporary PostgreSQL container for testing.
/// Returns the connection string.
pub fn start_test_postgres() -> String {
    // For now, we assume the user has run:
    // docker compose -p marketplace -f compose.postgres.yml up -d
    // and we read the connection string from environment or default.
    let db_url = env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/marketplace_test".to_string());
    // In a real setup, we might spin up a container here and tear it down.
    db_url
}

/// Build the Actix server binary in test mode.
/// This is a placeholder; in reality we would start the server as a subprocess or in-thread.
pub fn start_test_server() -> std::process::Child {
    // Example: cargo run --bin marketplace_server -- --test-mode
    Command::new("cargo")
        .args(&["run", "--bin", "marketplace_server", "--", "--test-mode"])
        .spawn()
        .expect("failed to start test server")
}

/// Create a reqwest client configured to talk to the test server.
pub fn test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed to build test client")
}