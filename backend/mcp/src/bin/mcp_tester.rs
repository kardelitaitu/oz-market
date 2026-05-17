//! MCP Tester Binary
//!
//! Exercises the Marketplace MCP server over stdio and logs a small
//! launch-and-smoke-test report to stdout and `mcp_test.log`.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

struct LaunchConfig {
    command: String,
    claims_json: String,
    database_url: Option<String>,
}

impl LaunchConfig {
    fn resolve() -> Result<Self, Box<dyn std::error::Error>> {
        let command =
            std::env::var("MARKETPLACE_MCP_COMMAND").unwrap_or_else(|_| "marketplace-mcp".into());
        let claims_json = std::env::var("MARKETPLACE_MCP_CLAIMS_JSON").unwrap_or_else(|_| {
            marketplace_mcp::dev_launcher_claims_json()
                .expect("failed to serialize built-in dev launcher claims")
        });
        let database_url = std::env::var("MARKETPLACE_MCP_DATABASE_URL")
            .ok()
            .and_then(|value| {
                let value = value.trim().to_string();
                if value.is_empty() {
                    None
                } else {
                    Some(value)
                }
            });

        Ok(Self {
            command,
            claims_json,
            database_url,
        })
    }

    fn env_pairs(&self) -> Vec<(&str, &str)> {
        let mut pairs = vec![("MARKETPLACE_MCP_CLAIMS_JSON", self.claims_json.as_str())];
        if let Some(database_url) = self.database_url.as_deref() {
            pairs.push(("MARKETPLACE_MCP_DATABASE_URL", database_url));
        }
        pairs
    }
}

struct McpTestClient {
    child: Option<Child>,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout_reader: BufReader<std::process::ChildStdout>,
}

impl McpTestClient {
    fn new(config: &LaunchConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut command = Command::new(&config.command);
        command.env_remove("DATABASE_URL");
        for (key, value) in config.env_pairs() {
            command.env(key, value);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = command.spawn()?;

        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;

        Ok(Self {
            child: Some(child),
            stdin: BufWriter::new(stdin),
            stdout_reader: BufReader::new(stdout),
        })
    }

    fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let request_str = serde_json::to_string(&request)? + "\n";
        println!("Sending: {}", request_str.trim());

        self.stdin.write_all(request_str.as_bytes())?;
        self.stdin.flush()?;

        let mut response_str = String::new();
        self.stdout_reader.read_line(&mut response_str)?;
        println!("Received: {}", response_str.trim());

        let response: Value = serde_json::from_str(&response_str)?;
        Ok(response)
    }

    fn send_notification(&mut self, method: &str) -> Result<(), Box<dyn std::error::Error>> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method
        });

        let notification_str = serde_json::to_string(&notification)? + "\n";
        println!("Sending: {}", notification_str.trim());

        self.stdin.write_all(notification_str.as_bytes())?;
        self.stdin.flush()?;
        println!("Received: <none>");
        Ok(())
    }

    fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let params = json!({
            "name": tool_name,
            "arguments": arguments
        });

        let response = self.send_request("tools/call", params)?;

        if response.get("error").is_some() {
            let error_msg = format!("Tool call error: {:?}", response.get("error"));
            Err(error_msg.into())
        } else {
            Ok(response
                .get("result")
                .cloned()
                .unwrap_or(serde_json::Value::Null))
        }
    }
}

fn create_listing_request() -> Value {
    json!({
        "idempotency_key": "idem-create-mcp-1",
        "listing": {
            "schema_version": "1.0",
            "owner_id": "seller-1",
            "listing_type": "product",
            "category": "laptop",
            "title": "ThinkPad MCP Smoke Test",
            "condition": "used",
            "price": {
                "currency": "USD",
                "amount": 450.0
            },
            "location": {
                "country_code": "US",
                "country_name": "United States",
                "city": "Austin"
            },
            "picture_urls": [
                "https://example.com/item.jpg"
            ],
            "description": "Smoke test listing created by the MCP tester"
        }
    })
}

fn tool_json_value(result: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let content = result
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|items| items.first())
        .and_then(|first| first.get("text"))
        .and_then(|text| text.as_str())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No text content in tool result",
            )
        })?;
    let parsed = serde_json::from_str::<Value>(content)?;
    Ok(parsed)
}

impl Drop for McpTestClient {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

fn log_test(name: &str, passed: bool, details: &str) {
    let status = if passed { "PASS" } else { "FAIL" };
    let log_line = format!("[{}] {}: {}\n", status, name, details);

    print!("{}", log_line);

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("mcp_test.log")
        .and_then(|mut f| f.write_all(log_line.as_bytes()));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Marketplace MCP Tester ===\n");

    let _ = std::fs::remove_file("mcp_test.log");

    let config = LaunchConfig::resolve()?;
    println!("Using MCP sidecar command: {}", config.command);
    println!("Starting MCP server...");
    let mut client = McpTestClient::new(&config)?;
    println!("MCP server started.\n");

    thread::sleep(Duration::from_millis(500));

    println!("--- Test 1: Initialize ---");
    match client.send_request(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "clientInfo": {"name": "mcp-tester", "version": "1.0"}
        }),
    ) {
        Ok(_) => {
            let _ = client.send_notification("notifications/initialized");
            log_test("initialize", true, "Server initialized");
        }
        Err(e) => log_test("initialize", false, &e.to_string()),
    }

    println!("\n--- Test 2: List Tools ---");
    match client.send_request("tools/list", json!({})) {
        Ok(response) => {
            if let Some(result) = response.get("result") {
                if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                    println!("Available tools:");
                    let mut tool_names: Vec<String> = Vec::new();
                    for tool in tools {
                        if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                            println!("  - {}", name);
                            tool_names.push(name.to_string());
                        }
                    }

                    let expected = [
                        "create_listing",
                        "search_listings",
                        "get_listing",
                        "open_negotiation",
                        "submit_offer",
                        "get_negotiation_status",
                        "request_contact_reveal",
                        "approve_contact_reveal",
                        "accept_negotiation",
                        "reject_negotiation",
                    ];
                    let has_expected = expected
                        .iter()
                        .all(|name| tool_names.iter().any(|tool| tool == name));
                    log_test(
                        "list_tools",
                        has_expected && tool_names.len() == expected.len(),
                        &format!("Found {} tools", tool_names.len()),
                    );
                } else {
                    log_test("list_tools", false, "No tools array in result");
                }
            } else {
                log_test("list_tools", false, "No result in response");
            }
        }
        Err(e) => log_test("list_tools", false, &e.to_string()),
    }

    println!("\n--- Test 3: Create Listing ---");
    let created_listing_id = match client.call_tool("create_listing", create_listing_request()) {
        Ok(result) => match tool_json_value(&result) {
            Ok(payload) => {
                let listing_id = payload
                    .get("listing_id")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string());
                let status = payload.get("status").and_then(|value| value.as_str());
                if let Some(ref listing_id) = listing_id {
                    if status == Some("active") {
                        log_test(
                            "create_listing",
                            true,
                            &format!("Created listing {}", listing_id),
                        );
                    } else {
                        log_test(
                            "create_listing",
                            false,
                            "Missing listing_id or active status",
                        );
                    }
                } else {
                    log_test(
                        "create_listing",
                        false,
                        "Missing listing_id or active status",
                    );
                }
                listing_id
            }
            Err(e) => {
                log_test("create_listing", false, &e.to_string());
                None
            }
        },
        Err(e) => {
            log_test("create_listing", false, &e.to_string());
            None
        }
    };

    println!("\n--- Test 4: Search Listings ---");
    match client.call_tool(
        "search_listings",
        json!({
            "query": "ThinkPad",
            "limit": 10
        }),
    ) {
        Ok(result) => match tool_json_value(&result) {
            Ok(payload) => {
                if let Some(items) = payload.get("items").and_then(|i| i.as_array()) {
                    log_test(
                        "search_listings",
                        !items.is_empty(),
                        &format!("Found {} items", items.len()),
                    );
                } else {
                    log_test("search_listings", false, "No items array in response");
                }
            }
            Err(e) => log_test("search_listings", false, &e.to_string()),
        },
        Err(e) => log_test("search_listings", false, &e.to_string()),
    }

    println!("\n--- Test 5: Get Created Listing ---");
    if let Some(listing_id) = created_listing_id {
        match client.call_tool(
            "get_listing",
            json!({
                "listing_id": listing_id
            }),
        ) {
            Ok(result) => match tool_json_value(&result) {
                Ok(payload) => {
                    let got_listing_id = payload.get("listing_id").and_then(|value| value.as_str());
                    let got_status = payload.get("status").and_then(|value| value.as_str());
                    log_test(
                        "get_created_listing",
                        got_listing_id.is_some() && got_status == Some("active"),
                        "Fetched created listing",
                    );
                }
                Err(e) => log_test("get_created_listing", false, &e.to_string()),
            },
            Err(e) => log_test("get_created_listing", false, &e.to_string()),
        }
    } else {
        log_test(
            "get_created_listing",
            false,
            "Skipping because create_listing failed",
        );
    }

    println!("\n--- Test 6: Get Non-existent Listing ---");
    match client.call_tool(
        "get_listing",
        json!({
            "listing_id": "non-existent-id"
        }),
    ) {
        Ok(result) => {
            if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                if let Some(first) = content.first() {
                    if let Some(text) = first.get("text").and_then(|t| t.as_str()) {
                        if text.contains("not found") {
                            log_test(
                                "get_non_existent_listing",
                                true,
                                "Correctly returned not found",
                            );
                        } else {
                            log_test("get_non_existent_listing", false, "Unexpected response");
                        }
                    } else {
                        log_test("get_non_existent_listing", false, "No text in content");
                    }
                } else {
                    log_test("get_non_existent_listing", false, "Empty content");
                }
            } else {
                log_test("get_non_existent_listing", false, "No content in result");
            }
        }
        Err(e) => log_test("get_non_existent_listing", false, &e.to_string()),
    }

    println!("\n=== MCP Test Complete ===");
    println!("Log saved to: mcp_test.log");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_config_env_pairs_include_claims_and_optional_database_url() {
        let config = LaunchConfig {
            command: "marketplace-mcp".to_string(),
            claims_json: "{\"sub\":\"agent\"}".to_string(),
            database_url: Some("postgres://example".to_string()),
        };

        assert_eq!(
            config.env_pairs(),
            vec![
                ("MARKETPLACE_MCP_CLAIMS_JSON", "{\"sub\":\"agent\"}"),
                ("MARKETPLACE_MCP_DATABASE_URL", "postgres://example"),
            ]
        );
    }

    #[test]
    fn launch_config_env_pairs_skip_database_url_when_not_requested() {
        let config = LaunchConfig {
            command: "marketplace-mcp".to_string(),
            claims_json: "{\"sub\":\"agent\"}".to_string(),
            database_url: None,
        };

        assert_eq!(
            config.env_pairs(),
            vec![("MARKETPLACE_MCP_CLAIMS_JSON", "{\"sub\":\"agent\"}")]
        );
    }
}
