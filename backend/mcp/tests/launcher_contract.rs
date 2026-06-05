use std::process::Command;

#[test]
fn mcp_tester_spawns_the_real_sidecar_with_explicit_launcher_claims() {
    let claims_json = oz_market_mcp::dev_launcher_claims_json()
        .expect("failed to serialize built-in dev launcher claims");
    let sidecar = env!("CARGO_BIN_EXE_oz-market-mcp");
    let tester = env!("CARGO_BIN_EXE_mcp_tester");

    let output = Command::new(tester)
        .env("MARKETPLACE_MCP_COMMAND", sidecar)
        .env("MARKETPLACE_MCP_CLAIMS_JSON", claims_json)
        .output()
        .expect("failed to run mcp_tester");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "mcp_tester failed\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(stdout.contains("[PASS] initialize"));
    assert!(stdout.contains("[PASS] list_tools"));
    assert!(stdout.contains("[PASS] create_listing"));
    assert!(stdout.contains("[PASS] search_listings"));
    assert!(stdout.contains("[PASS] get_created_listing"));
}
