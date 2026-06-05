# oz-market-mobile — Tauri v2 + Svelte 5

A cross-platform mobile client (Android + iOS) for the marketplace backend. Built with Tauri v2, Svelte 5, and Rust — sharing types directly via the `oz-market-api-contract` crate.

## Prerequisites

- Rust toolchain (`rustup`, `cargo`)
- **Windows:** [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (included in Windows 10+)
- **Android:** Android SDK + NDK (`cargo tauri android init`)
- **iOS:** Xcode + macOS (`cargo tauri ios init`)

## Development

### 1. Start the Backend

```shell
# From project root — starts the server on http://127.0.0.1:3000
.\live-test.ps1
```

Or run the server directly:

```shell
cargo run --bin oz-market-server
```

### 2. Start the Frontend Dev Server

```shell
cd mobile/marketplace
npm install
npm run dev
```

The Vite dev server starts on `http://localhost:1420`.

### 3. Launch the Tauri App

```shell
# In a separate terminal
cargo tauri dev
```

> **Note:** On Windows, `cargo tauri build` may fail on the `beforeBuildCommand` step due to a subprocess exit-code issue. Workaround: run `npm run build` first, then build with `cargo tauri build --no-bundle`.

### Shortcut (Desktop)

```shell
npm run build && cargo tauri build --no-bundle
```

The binary is written to `src-tauri/target/release/oz-market-mobile.exe`.

## Live Testing

1. Start the backend (see above).
2. Launch the app.
3. Set the backend URL in Settings (default `http://127.0.0.1:3000`).
4. On first launch, the app calls `GET /health` to verify connectivity.
5. Login screen — enter any `user_id` to generate claims and log in.

### Smoke Test

```shell
.\scripts\check.ps1
```

This runs `cargo check`, `npm run build`, `cargo fmt --check`, and `cargo clippy`.

### Current Coverage

| Feature            | Status | Notes                     |
|--------------------|--------|---------------------------|
| Health check       | ✅     | Called via `GET /health`  |
| Login              | ✅     | Keychain-stored claims    |
| Settings           | ✅     | Backend URL config        |
| Search listings    | ✅     | Paginated, full-text      |
| Listing detail     | ✅     | Price, status, type, seller|
| My Listings        | ✅     | Owner-filtered search     |
| Create listing     | ✅     | Product/service/property  |
| Open negotiation   | ✅     | From listing detail page  |
| Negotiation thread | ✅     | Offer history, polling    |
| Submit offer       | ✅     | Counter-offer flow        |
| Accept/reject      | ✅     | With idempotency keys     |
| Request contact    | ✅     | Buyer-initiated reveal    |
| Approve reveal     | ⏳     | Needs reveal_id tracking  |
| Agent chat         | ✅     | Natural-language listing search |

## Workspace Layout

```
mobile/marketplace/
  src/                  # Svelte 5 frontend
    routes/             # SvelteKit pages
    lib/                # Shared Svelte modules
      api/commands.ts   # Tauri IPC invoke wrappers
  src-tauri/            # Rust backend shell
    src/
      commands/         # Tauri IPC commands
      client/           # HTTP client wrappers
      auth/             # Auth + keychain
    Cargo.toml          # Rust dependencies
  scripts/              # Build and test helpers
```

## Tauri Commands

| Command                | Args                          | Returns                  |
|------------------------|-------------------------------|--------------------------|
| `health`               | —                             | `serde_json::Value`      |
| `get_listing`          | `listing_id`                  | `ListingSummary`         |
| `search_listings`      | `query, category, limit, cursor` | `SearchResponse`      |
| `my_listings`          | `limit, cursor`               | `SearchResponse`         |
| `create_listing`       | `CreateListingParams`         | `ListingSummary`         |
| `login`                | `sub, roles, scopes, ...`     | `String` (claims)        |
| `logout`               | —                             | `bool`                   |
| `get_claims`           | —                             | `String`                 |
| `set_base_url`         | `url`                         | `bool`                   |
| `get_base_url`         | —                             | `String`                 |
| `open_negotiation`     | `OpenNegotiationParams`       | `NegotiationResponse`    |
| `get_negotiation`      | `negotiation_id`              | `NegotiationResponse`    |
| `submit_offer`         | `OfferParams`                 | `NegotiationResponse`    |
| `accept_negotiation`   | `negotiation_id, idempotency_key` | `NegotiationResponse` |
| `reject_negotiation`   | `negotiation_id, idempotency_key` | `NegotiationResponse` |
| `request_contact_reveal`| `negotiation_id, idempotency_key` | `ContactRevealResponse` |
| `approve_contact_reveal`| `reveal_id, idempotency_key` | `ContactRevealResponse`  |
| `agent_query`          | `query, conversation_id`     | `AgentQueryResponse`      |

## Further Reading

- [`docs/mobile/ROADMAP.md`](../../docs/mobile/ROADMAP.md) — milestone plan
- [`docs/mobile/tauri-svelte-plan.md`](../../docs/mobile/tauri-svelte-plan.md) — architecture decisions
- [`docs/server/module-layout.md`](../../docs/server/module-layout.md) — backend structure
