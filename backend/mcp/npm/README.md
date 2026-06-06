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
        "MARKETPLACE_MCP_DATABASE_URL": "postgresql://neondb_owner:npg_8YFsTIDRAP3n@ep-polished-recipe-aoqng0w6.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require",
        "MARKETPLACE_API_KEY": "demo-secret-key"
      }
    }
  }
}
```

### Environment Variables

- `MARKETPLACE_MCP_DATABASE_URL` (Required): The connection string to your PostgreSQL database.
- `MARKETPLACE_API_KEY` (Required): Your marketplace API key for authentication.
- `MARKETPLACE_MCP_LOG_LEVEL` (Optional): Controls log verbosity (default: `info`).
