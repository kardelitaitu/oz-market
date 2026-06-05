use serde_json::{json, Value};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};

struct BasicMcpClient {
    child: Option<Child>,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout_reader: BufReader<std::process::ChildStdout>,
}

impl BasicMcpClient {
    fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let sidecar = env!("CARGO_BIN_EXE_oz-market-mcp");
        let claims_json = oz_market_mcp::dev_launcher_claims_json()
            .expect("failed to serialize built-in dev launcher claims");

        let mut command = Command::new(sidecar);
        command
            .env("MARKETPLACE_MCP_CLAIMS_JSON", claims_json)
            .env_remove("DATABASE_URL")
            .env_remove("MARKETPLACE_MCP_DATABASE_URL")
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
        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("basic-{}", method),
            "method": method,
            "params": params,
        });

        let request_line = serde_json::to_string(&request)? + "\n";
        self.stdin.write_all(request_line.as_bytes())?;
        self.stdin.flush()?;

        let mut response_line = String::new();
        self.stdout_reader.read_line(&mut response_line)?;
        Ok(serde_json::from_str(&response_line)?)
    }

    fn send_notification(&mut self, method: &str) -> Result<(), Box<dyn std::error::Error>> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
        });

        let notification_line = serde_json::to_string(&notification)? + "\n";
        self.stdin.write_all(notification_line.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }
}

impl Drop for BasicMcpClient {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

#[test]
fn oz_market_mcp_boots_and_lists_the_public_tool_catalog() {
    let mut client = BasicMcpClient::spawn().expect("failed to spawn oz-market-mcp");

    let initialize = client
        .send_request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "clientInfo": {"name": "basic-mcp-test", "version": "1.0"}
            }),
        )
        .expect("initialize request failed");
    assert!(
        initialize.get("error").is_none(),
        "initialize returned error"
    );
    assert!(
        initialize.get("result").is_some(),
        "initialize returned no result"
    );

    client
        .send_notification("notifications/initialized")
        .expect("initialized notification failed");

    let tools_response = client
        .send_request("tools/list", json!({}))
        .expect("tools/list request failed");
    assert!(
        tools_response.get("error").is_none(),
        "tools/list returned error"
    );

    let tools = tools_response
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(|tools| tools.as_array())
        .expect("tools/list response did not include tools");

    let tool_names: Vec<String> = tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(|name| name.as_str()))
        .map(|name| name.to_string())
        .collect();

    let expected = [
        "agent_query",
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

    assert_eq!(
        tool_names.len(),
        expected.len(),
        "unexpected tool count: {:?}",
        tool_names
    );
    for name in expected {
        assert!(
            tool_names.iter().any(|tool| tool == name),
            "missing expected tool: {name}"
        );
    }
    assert!(
        !tool_names.iter().any(|tool| tool == "get_contact_reveal"),
        "internal helper leaked into public tool catalog"
    );
}
