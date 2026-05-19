# Tauri + Svelte Mobile App Plan

## Overview

Unified Android + iOS app using **Tauri v2** (Rust shell) + **Svelte 5** (frontend). Shares `marketplace-api-contract` crate types directly with the backend — no codegen, no drift.

**Why Tauri + Svelte over native:**
- Shared Rust types across backend and mobile (single source of truth)
- One codebase for Android + iOS + desktop
- Svelte compiles away at build time (~2KB runtime, no virtual DOM)
- Tiny bundle size (~3MB vs 20-50MB for Flutter/RN/native)
- Team only needs Rust + Svelte (no Kotlin/Swift required)

**Key constraints** (inherited from V1 contract):
- Same backend contract as HTTP and MCP — no mobile-only payload variants
- Authz and abuse controls stay server-side
- Polling-first event model (no mobile event stream in V1)
- Idempotency keys for all replay-sensitive writes

---

## Architecture

```
mobile/marketplace/
├── src-tauri/                  # Rust desktop/mobile shell (Tauri)
│   ├── src/
│   │   ├── main.rs             # Tauri app entry point
│   │   ├── lib.rs              # Plugin registration, app setup
│   │   ├── commands/           # Tauri IPC commands
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs         # login, logout, token refresh
│   │   │   ├── listings.rs     # create, get, search
│   │   │   ├── negotiations.rs # open, offer, accept, reject
│   │   │   └── contact.rs      # request reveal, approve
│   │   ├── client/             # HTTP API client
│   │   │   ├── mod.rs
│   │   │   ├── requests.rs     # typed request builders
│   │   │   └── responses.rs    # response deserialization
│   │   ├── auth/               # token storage, claims parsing
│   │   │   ├── mod.rs
│   │   │   └── keystore.rs     # platform keychain/keystore
│   │   └── state.rs            # Tauri managed state
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── icons/
│   ├── build.rs
│   └── gen/
│       └── schemas/            # auto-generated Tauri command schemas
├── src/                        # Svelte 5 frontend
│   ├── app.html                # HTML shell
│   ├── main.ts                 # Svelte mount
│   ├── app.css                 # global styles
│   ├── lib/
│   │   ├── api/
│   │   │   ├── commands.ts     # invoke wrappers for Tauri commands
│   │   │   └── types.ts        # TypeScript types (generated from Rust)
│   │   ├── stores/
│   │   │   ├── auth.ts         # login state, token
│   │   │   └── listings.ts     # cached listings
│   │   ├── components/
│   │   │   ├── ListingCard.svelte
│   │   │   ├── PriceBadge.svelte
│   │   │   ├── NegotiationStatus.svelte
│   │   │   └── ...
│   │   └── utils/
│   │       ├── format.ts       # currency, date formatting
│   │       └── idempotency.ts  # idempotency key generation
│   └── routes/                 # SvelteKit file-based routing (if used)
│       ├── +layout.svelte
│       ├── +page.svelte        # home / search
│       ├── listings/
│       │   ├── [id]/
│       │   │   └── +page.svelte  # listing detail
│       │   └── create/
│       │       └── +page.svelte  # create listing
│       ├── negotiations/
│       │   └── [id]/
│       │       └── +page.svelte  # negotiation thread
│       ├── contact/
│       │   └── [id]/
│       │       └── +page.svelte  # contact reveal
│       └── settings/
│           └── +page.svelte
├── package.json
├── svelte.config.js
├── vite.config.ts
├── tsconfig.json
└── static/
    └── favicon.png
```

---

## Rust Side (src-tauri)

### Shared Crate Integration

`src-tauri/Cargo.toml` depends on the workspace crate:

```toml
[dependencies]
marketplace-api-contract = { path = "../../backend/crates/api-contract" }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
keyring = "3"                  # platform keychain
tokio = { version = "1", features = ["full"] }
```

No codegen needed — types are shared directly via the crate.

### API Client

The `client/` module wraps `reqwest::Client` and uses `marketplace-api-contract` types directly:

```rust
// client/requests.rs
pub async fn create_listing(
    client: &Client,
    base_url: &str,
    claims: &Claims,
    request: &CreateListingRequest,
) -> Result<CreateListingResponse, ApiError> {
    let idempotency_key = request.idempotency_key.clone();
    let resp = client
        .post(format!("{base_url}/v1/listings"))
        .header("x-marketplace-claims", serialize_claims(claims))
        .json(&request)
        .send()
        .await?;
    // handle 200 (replay) and 201 (first-use) identically
    Ok(resp.json::<CreateListingResponse>().await?)
}
```

### Tauri Commands

Commands are thin wrappers that extract managed state and delegate to the client:

```rust
#[tauri::command]
async fn search_listings(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<SearchResponse, String> {
    let request = SearchRequest { query, limit, .. };
    state.client.search_listings(&request).await.map_err(|e| e.to_string())
}
```

### Auth & Token Storage

- JWT claims are stored in the platform keychain (via `keyring` crate)
- `x-marketplace-claims` header is reconstructed from stored claims
- Token refresh is handled by a Tauri command, called when the backend returns 401

### Idempotency Keys

Generated on the Rust side using `uuid::Uuid::v4()` and passed up to the Svelte UI as needed. The Rust client automatically generates and attaches keys for write operations if not provided.

---

## Svelte Side (src/)

### Frontend Framework Choice

**SvelteKit** (not vanilla Svelte) gives us file-based routing, SSR/SSG for loading states, and a built-in fetch wrapper. For a mobile app, we use the **static adapter** (`@sveltejs/adapter-static`) to produce a pure SPA that Tauri loads via webview.

### Tauri API Bridge

All backend calls go through `@tauri-apps/api` invoke:

```typescript
// lib/api/commands.ts
import { invoke } from '@tauri-apps/api/core';

export async function searchListings(query: string, limit = 20) {
  return invoke<SearchResponse>('search_listings', { query, limit });
}
```

TypeScript types mirror the Rust contract types — generated once via `ts-rs` or typed manually from the contract.

### Screens

| Route | Screen | Tauri Commands Used |
|-------|--------|-------------------|
| `/` | Home / Search bar + recent listings | `search_listings` |
| `/listings/[id]` | Listing detail, open negotiation | `get_listing`, `open_negotiation` |
| `/listings/create` | Create listing form | `create_listing` |
| `/negotiations/[id]` | Offer thread, accept/reject, contact reveal | `get_negotiation`, `submit_offer`, `accept_negotiation`, `reject_negotiation`, `request_contact_reveal` |
| `/contact/[id]` | Contact info display | `approve_contact_reveal` (seller side) |
| `/settings` | Account, logout, about | `logout` |

### State Management

Svelte 5 runes (`$state`, `$derived`, `$effect`) replace stores:

```svelte
<script lang="ts">
let listings = $state<ListingSummary[]>([]);
let loading = $state(true);

$effect(() => {
  searchListings($searchQuery).then(r => {
    listings = r.results;
    loading = false;
  });
});
</script>
```

---

## Auth Flow

```
┌─────────┐     ┌──────────┐     ┌──────────┐
│ Svelte  │────▶│ Tauri    │────▶│ Backend  │
│ UI      │     │ Rust cmd │     │ API      │
└─────────┘     └──────────┘     └──────────┘
     │               │               │
     │  login()      │               │
     │──────────────▶│               │
     │               │ POST /auth    │
     │               │──────────────▶│
     │               │ 200 + JWT     │
     │               │◀──────────────│
     │               │ store in      │
     │               │ keychain      │
     │  {ok}         │               │
     │◀──────────────│               │
     │               │               │
     │  invoke(cmd)  │               │
     │──────────────▶│──────────────▶│
     │               │               │
     │               │ 401           │
     │               │◀──────────────│
     │               │ refresh?      │
     │               │ or re-login   │
     │  401 error    │               │
     │◀──────────────│               │
```

Claims are stored in the platform keystore (Keychain on iOS, KeyStore on Android) via the `keyring` crate. The Tauri command layer deserializes them into `Claims` structs and attaches the `x-marketplace-claims` header on every request.

---

## Phase Plan

### Phase 1 — Project Scaffold

**Goal:** Runnable Tauri + Svelte app showing a shell screen.

- `npm create tauri-app@latest` with Svelte template
- Configure `tauri.conf.json` for Android + iOS targets
- Set up workspace dependency on `marketplace-api-contract`
- Create `src-tauri/src/state.rs`, `commands/mod.rs`, `client/mod.rs` skeletons
- Create SvelteKit app with `@sveltejs/adapter-static`
- Wire Tauri IPC invoke from Svelte
- Acceptance: `cargo tauri android dev` launches on emulator, Svelte shell renders

### Phase 2 — API Client & Auth

**Goal:** Authenticated HTTP calls from mobile.

- Implement `client/` module with typed reqwest wrappers for all 11 endpoints
- Implement `auth/` module with login, token storage, claims reconstruction
- Wire `x-marketplace-claims` header into every request
- Create Tauri commands for all read endpoints (health, get listing, search)
- Svelte: login screen, token store, authenticated invoke wrappers
- Acceptance: app fetches and displays search results from live API

### Phase 3 — Core Screens

**Goal:** Browse, search, and create listings.

- Listing detail screen with price, location, images
- Search screen with query input and results list
- Create listing form with validation
- Tauri commands for `create_listing`, `get_listing`, `search_listings`
- Svelte: forms, validation, loading/error states
- Acceptance: full CRUD lifecycle for listings from mobile

### Phase 4 — Negotiation & Contact Reveal

**Goal:** Complete negotiation flow on mobile.

- Negotiation thread screen with offer history
- Submit counter-offer, accept, reject
- Request contact reveal (buyer) and approve (seller)
- Svelte: real-time-ish polling for status updates
- Tauri commands for all negotiation + reveal endpoints
- Acceptance: end-to-end negotiation + contact reveal flow works

### Phase 5 — AI Agent Integration

**Goal:** AI-powered search and negotiation suggestions.

- Agent chat UI component
- Tauri commands that call MCP or directly use the agent endpoints
- Svelte: chat UI with streaming-like experience
- Acceptance: user can ask "find laptops under $1000" and the agent returns results

### Phase 6 — Production Polish

**Goal:** Store-ready release.

- Error boundaries and offline state
- Push notifications (via Tauri notification plugin)
- Deep linking
- CI/CD pipeline (GitHub Actions for Tauri mobile builds)
- App icons, splash screen, store metadata
- Acceptance: `cargo tauri build` produces signed APK and IPA

---

## Comparison: Tauri vs Native

| Dimension | Tauri + Svelte | Native (Kotlin/Swift) |
|-----------|---------------|----------------------|
| Code sharing | 100% between platforms | ~60% (shared logic, separate UI) |
| Rust type sharing | Direct via crate | OpenAPI codegen |
| Bundle size | ~3MB | ~15-30MB |
| UI fidelity | WebView (good, not native) | Native widgets |
| Team expertise | Rust + Svelte | Kotlin + Swift |
| Dev speed | ~2x faster (one codebase) | ~1x (two codebases) |
| Platform-specific features | Plugin gap possible | Full access |

---

## Decision Record

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Framework | Tauri v2 | Rust-native mobile shell, shared types with backend |
| Frontend | Svelte 5 | Smallest bundle, compiles away, no virtual DOM |
| Routing | SvelteKit + static adapter | File-based routing, zero-config |
| HTTP client | reqwest 0.12 | Already in workspace, async, JSON-first |
| Auth storage | `keyring` crate | Cross-platform keychain/keystore access |
| Type bridge | Manual TS from contract | Simple enough for V1; `ts-rs` if friction appears |
