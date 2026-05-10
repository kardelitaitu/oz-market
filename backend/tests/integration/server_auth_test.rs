use crate::utils::{start_test_server, test_client, start_test_postgres};
use std::process::Child;
use std::thread::sleep;
use std::time::Duration;

/// Integration test for the HTTP server's auth endpoint.
/// This test starts the server binary in a child process, waits a short moment
/// for it to bind, then issues a request to `/auth/validate` (placeholder).
///
/// The test is marked `#[ignore]` because the server may not be fully
/// implemented yet. Once the endpoint exists, remove the attribute.
#[ignore]
#[test]
fn test_auth_validate_endpoint() {
    // Ensure test DB is ready (placeholder – uses env var or default)
    let _db_url = start_test_postgres();

    // Launch the server in test mode
    let mut server: Child = start_test_server();
    // Give the server time to start listening (adjust as needed)
    sleep(Duration::from_secs(2));

    let client = test_client();
    // Replace with actual URL/port from server's config if needed
    let resp = client
        .get("http://127.0.0.1:8080/auth/validate")
        .header("Authorization", "Bearer dummy.token")
        .send()
        .expect("failed to send request");

    assert!(resp.status().is_success(), "expected success response");

    // Clean up the server process
    let _ = server.kill();
}
