# @kardelitaitu/oz-market-mcp

NPM wrapper for the Rust-based `oz-market` Model Context Protocol (MCP) server. This package allows you to run the MCP server using `npx` without needing to build the Rust binary from source.

## Configuration for Claude Desktop

Add the following to your Claude Desktop configuration file (`%APPDATA%\Claude\claude_desktop_config.json` on Windows):

```json
{
  "mcpServers": {
    "oz-market": {
      "command": "npx",
      "args": [
        "-y",
        "@kardelitaitu/oz-market-mcp"
      ],
      "env": {
        "MARKETPLACE_API_KEY": "demo-secret-key"
      }
    }
  }
}
```

### Environment Variables

- `MARKETPLACE_API_KEY` (Required): Your marketplace API key for authentication.
- `MARKETPLACE_MCP_DATABASE_URL` (Optional): Custom PostgreSQL database connection string. Defaults to the live production database. Can be set to `in-memory` for offline testing.
- `MARKETPLACE_MCP_LOG_LEVEL` (Optional): Controls log verbosity (default: `info`).
