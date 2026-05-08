# MCP Documentation

This folder contains marketplace MCP server documentation.

## Intended Contents:

- MCP tool catalog
- tool input/output contracts
- auth flow for desktop agents
- MCP examples
- failure and retry behavior

## Current Status:

The **MCP server is implemented and compiles! 🎉**

| Component | Status | Details |
|-----------|--------|---------|
| `marketplace-mcp` package | ✅ **COMPILES!** | Binary built: `target/debug/marketplace-mcp.exe` (14MB) |
| `mcp/src/lib.rs` | ✅ Complete | 7 MCP tools implemented |
| `mcp/src/bin/mcp_tester.rs` | ⚠️ Has issues | Type inference errors in closures |

## MCP Tools Implemented (7 tools):

| Tool | Purpose | Required Role |
|------|---------|---------------|
| `create_listing` | Create seller listing | `seller_listing_writer` |
| `search_listings` | Search indexed listings | `buyer_searcher` or seller-side role |
| `get_listing` | Fetch one listing | authenticated client |
| `open_negotiation` | Open buyer-side negotiation | `buyer_negotiator` |
| `request_contact_reveal` | Request contact reveal | `buyer_negotiator` |
| `approve_contact_reveal` | Seller-side approval | `seller_contact_reveal_approver` |
| `get_negotiation_status` | Fetch negotiation state | authorized participant |

## How to Use with Claude Desktop:

1. **Build the MCP server**:
   ```bash
   cd backend && cargo build --package marketplace-mcp
   ```

2. **Configure Claude Desktop** (settings.json):
   ```json
   {
     "mcpServers": {
       "marketplace": {
         "command": "/path/to/marketplace-mcp.exe"
       }
     }
   }
   ```

3. **AI can now use tools**:
   - "Search for laptops under $500"
   - "Create a new listing for my ThinkPad"
   - "Check negotiation status for neg_123"

## MCP Implementation Details:

### Architecture:
```
marketplace-mcp (crate)
├── lib.rs - MCP server implementation
│   ├── MarketplaceMcpServer (implements ServerHandler)
│   ├── MarketplaceMcp (wraps MarketplaceApp)
│   └── MCP tools (7 tools via #[tool_router])
└── bin/
    └── mcp_tester.rs (has compilation issues)
```

### Transport:
- **stdio** transport for desktop agents (Claude Desktop, Cursor, etc.)
- MCP server communicates via stdin/stdout
- Each tool delegates to `MarketplaceApp` methods (same as HTTP API!)

### Claims Handling:
- Each tool builds appropriate `Claims` (roles + scopes)
- Pre-configured for each tool's required permissions
- Uses `marketplace-auth-core` for auth logic

## Known Issues:

### 1. MCP Tester Has Compilation Errors ⚠️
- **File**: `mcp/src/bin/mcp_tester.rs`
- **Issue**: Type inference in `and_then()` closure chains
- **Workaround**: Test MCP server manually:
  ```bash
  # Terminal 1: Start MCP server
  cd backend && cargo run --package marketplace-mcp
  
  # Terminal 2: Send JSON-RPC manually
  echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {...}}' | ./target/debug/marketplace-mcp.exe
  ```

### 2. Pre-existing Server Errors (Now FIXED ✅):
- ✅ `reservations` module added to `services/mod.rs`
- ✅ `moka::sync` feature enabled in `server/Cargo.toml`
- ✅ Broken `#[path(...)]` attributes removed from `actix_handlers.rs`

## Next Docs to Add:

1. ✅ Auth and session flow for MCP clients
2. ✅ Example desktop agent workflows
3. ✅ Conflict and retry examples
4. ✅ MCP tool input/output schemas (reference tool-catalog.md)

## References:

- `tool-catalog.md` - Detailed tool definitions
- `../whitepaper/07-mcp-server.md` - MCP design document
- `../whitepaper/10-api-contract.md` - API contract (shared with HTTP)
- `../specs/openapi.yaml` - Full API specification (20+ endpoints)

---

**The MCP server is ready!** Just needs testing with a real MCP client. 🚀
