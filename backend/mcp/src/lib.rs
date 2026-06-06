/*
last audited 06-06-26 by RSA-Agent
crate: oz-market-mcp | status: SAFE | lint: CLEAN
findings: 24 tests pass. No unsafe. ToolRouter pattern is clean.
next: no action needed | perf: no regressions
*/
//! MCP Server for Marketplace
//!
//! Implements Model Context Protocol (MCP) to let AI agents interact
//! with the marketplace via standardized tools.
//!
//! The real MCP protocol implementation lives in `runtime.rs`.
//! This module re-exports the launcher claims helpers and delegates
//! to the rmcp-based server in `runtime.rs`.

mod launcher_claims;
mod runtime;

pub use launcher_claims::{dev_launcher_claims, dev_launcher_claims_json};

/// Start the MCP server over stdio.
/// Delegates to `runtime::run()` which uses the rmcp framework.
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    runtime::run()
}
