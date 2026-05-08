//! MCP Tester Binary
//! 
//! Tests the Marketplace MCP server by sending JSON-RPC requests
//! and validating responses. Logs results to stdout and mcp_test.log.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

struct McpTestClient {
    child: Child,
    stdin: std::process::ChildStdin,
    stdout_reader: BufReader<std::process::ChildStdout>,
}

impl McpTestClient {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Start the marketplace-mcp binary (assumes it's built)
        let mut child = Command::new("marketplace-mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stdout_reader = BufReader::new(stdout);
        
        Ok(Self { child, stdin, stdout_reader })
    }
    
    fn send_request(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let id = chrono::Utc::now().timestamp_millis();
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
        
        // Read response (assuming line-based)
        let mut response_str = String::new();
        self.stdout_reader.read_line(&mut response_str)?;
        
        println!(" ← Received: {}", response_str.trim());
        
        let response: Value = serde_json::from_str(&response_str)?;
        Ok(response)
    }
    
    fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let params = json!({
            "name": tool_name,
            "arguments": arguments
        });
        
        let response = self.send_request("tools/call", params)?;
        
        if let Some(error) = response.get("error") {
            Err(format!("Tool call error: {}", error).into())
        } else {
            Ok(response.get("result").cloned().unwrap_or(json!(null)))
        }
    }
}

impl Drop for McpTestClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn log_test(name: &str, passed: bool, details: &str) {
    let status = if passed { "PASS" } else { "FAIL" };
    let log_line = format!("[{}] {}: {}\n", status, name, details);
    
    // Print to stdout
    print!("{}", log_line);
    
    // Append to log file
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
    
    // Test 1: Ping (initialize)
    println!("--- Test 1: Initialize ---");
    match client.send_request("initialize", json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {"tools": {}},
        "clientInfo": {"name": "mcp-tester", "version": "1.0"}
    })) {
        Ok(_) => log_test("initialize", true, "Server initialized"),
        Err(e) => log_test("initialize", false, &e.to_string()),
    }
    
    // Test 2: List tools
    println!("\n--- Test 2: List Tools ---");
    match client.send_request("tools/list", json!({})) {
        Ok(response) => {
            if let Some(tools) = response.get("result").and_then(|r| r.get("tools")) {
                let tool_names: Vec<&str> = tools.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|t| t.get("name"))
                    .filter_map(|n| n.as_str())
                    .collect();
                
                println!("Available tools: {:?}", tool_names);
                let expected_tools = vec!["create_listing", "search_listings", "get_listing", "open_negotiation", "request_contact_reveal", "approve_contact_reveal", "get_negotiation_status"];
                let has_all = expected_tools.iter().all(|t| tool_names.contains(t));
                log_test("list_tools", has_all, &format!("Found {} tools", tool_names.len()));
            } else {
                log_test("list_tools", false, "No tools in response");
            }
        }
        Err(e) => log_test("list_tools", false, &e.to_string()),
    }
    
    // Test 3: Search listings (empty query)
    println!("\n--- Test 3: Search Listings ---");
    match client.call_tool("search_listings", json!({
        "query": null,
        "category": null,
        "condition": null,
        "min_seller_rating": null,
        "sort_by": null,
        "limit": 10
    })) {
        Ok(result) => {
            if let Some(items) = result.get("content").and_then(|c| c.get(0)).and_then(|c| c.get("text")) {
                if let Ok(search_response) = serde_json::from_str::<Value>(items.as_str().unwrap_or("")) {
                    if let Some(items_array) = search_response.get("items").and_then(|i| i.as_array()) {
                        log_test("search_listings", true, &format!("Found {} items", items_array.len()));
                    } else {
                        log_test("search_listings", false, "No items array in response");
                    }
                } else {
                    log_test("search_listings", false, "Failed to parse search response");
                }
            } else {
                log_test("search_listings", false, "No content in tool result");
            }
        }
        Err(e) => log_test("search_listings", false, &e.to_string()),
    }
    
    // Test 4: Get non-existent listing
    println!("\n--- Test 4: Get Non-existent Listing ---");
    match client.call_tool("get_listing", json!({
        "listing_id": "non-existent-id"
    })) {
        Ok(result) => {
            if let Some(content) = result.get("content").and_then(|c| c.get(0)).and_then(|c| c.get("text")) {
                if content.as_str().unwrap_or("") == "Listing not found" {
                    log_test("get_non_existent_listing", true, "Correctly returned not found");
                } else {
                    log_test("get_non_existent_listing", false, "Unexpected response");
                }
            } else {
                log_test("get_non_existent_listing", false, "No content in tool result");
            }
        }
        Err(e) => log_test("get_non_existent_listing", false, &e.to_string()),
    }
    
    println!("\n=== MCP Test Complete ===");
    println!("Log saved to: mcp_test.log");
    
    Ok(())
}
