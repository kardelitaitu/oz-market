//! MCP Tester Binary - Super Simple
//!
//! Tests the Marketplace MCP server by sending JSON-RPC requests.
//! Logs results to stdout and mcp_test.log.

use serde_json::{json, Value};
use std::io::{BufRead, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

struct McpTestClient {
    child: Option<Child>,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout_reader: BufReader<std::process::ChildStdout>,
}

impl McpTestClient {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut child = Command::new("marketplace-mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stdout_reader = BufReader::new(stdout);
        let stdin_writer = BufWriter::new(stdin);

        Ok(Self {
            child: Some(child),
            stdin: stdin_writer,
            stdout_reader,
        })
    }

    fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let request_str = serde_json::to_string(&request)? + "\n";
        print!("→ Sending: {}", request_str.trim());

        self.stdin.write_all(request_str.as_bytes())?;
        self.stdin.flush()?;

        let mut response_str = String::new();
        self.stdout_reader.read_line(&mut response_str)?;

        println!(" ← Received: {}", response_str.trim());

        let response: Value = serde_json::from_str(&response_str)?;
        Ok(response)
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
            match response.get("result") {
                Some(result) => Ok(result.clone()),
                None => Ok(json!(null)),
            }
        }
    }
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

    // Clear previous log
    let _ = std::fs::remove_file("mcp_test.log");

    println!("Starting MCP server...");
    let mut client = McpTestClient::new()?;
    println!("MCP server started.\n");

    // Give server time to initialize
    thread::sleep(Duration::from_millis(500));

    // Test 1: Initialize
    println!("--- Test 1: Initialize ---");
    match client.send_request(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "clientInfo": {"name": "mcp-tester", "version": "1.0"}
        }),
    ) {
        Ok(_) => log_test("initialize", true, "Server initialized"),
        Err(e) => log_test("initialize", false, &e.to_string()),
    }

    // Test 2: List tools
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
                    log_test(
                        "list_tools",
                        true,
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

    // Test 3: Search listings (empty query)
    println!("\n--- Test 3: Search Listings ---");
    match client.call_tool(
        "search_listings",
        json!({
            "query": null,
            "limit": 10
        }),
    ) {
        Ok(result) => {
            if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                if let Some(first) = content.first() {
                    if let Some(text) = first.get("text").and_then(|t| t.as_str()) {
                        if let Ok(search_response) = serde_json::from_str::<Value>(text) {
                            if let Some(items) =
                                search_response.get("items").and_then(|i| i.as_array())
                            {
                                log_test(
                                    "search_listings",
                                    true,
                                    &format!("Found {} items", items.len()),
                                );
                            } else {
                                log_test("search_listings", false, "No items array in response");
                            }
                        } else {
                            log_test("search_listings", false, "Failed to parse search response");
                        }
                    } else {
                        log_test("search_listings", false, "No text in content");
                    }
                } else {
                    log_test("search_listings", false, "Empty content array");
                }
            } else {
                log_test("search_listings", false, "No content in tool result");
            }
        }
        Err(e) => log_test("search_listings", false, &e.to_string()),
    }

    // Test 4: Get non-existent listing
    println!("\n--- Test 4: Get Non-existent Listing ---");
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
