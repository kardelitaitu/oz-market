# Mobile App Development Roadmap

**Platform:** Tauri v2 + Svelte 5 (Android + iOS from one codebase)  
**API:** HTTP JSON against `docs/specs/openapi.yaml` (no MCP/stdio on mobile)  
**Rust types:** Shared via `oz-market-api-contract` crate — zero codegen, zero drift  

---

## Vision

> A lightweight, responsive mobile app for the marketplace that lets buyers and sellers discover listings, negotiate in real-time, and complete contact reveals — all powered by the same Rust types and backend contract as the web and MCP surfaces.

---

## Milestones

### M1 — Foundation (Weeks 1–2)

| Week | Deliverable | Depends On |
|------|-------------|------------|
| 1 | Tauri + Svelte project boots on Android emulator and iOS simulator | `oz-market-api-contract` published as path dependency |
| 1 | Svelte shell renders, Tauri IPC invoke bridge works | — |
| 2 | `client/` module: typed reqwest wrappers for all 11 endpoints | M1 week 1 |
| 2 | `auth/` module: login, keychain storage, claims header injection | M1 week 2 |
| 2 | `commands/`: Tauri IPC commands for all read endpoints (health, get listing, search) | M1 week 2 |

**Gate:** `cargo tauri android dev` and `cargo tauri ios dev` both show a login screen that successfully calls the health endpoint.

### M2 — Core Listing Flow ✅

| Week | Deliverable | Depends On |
|------|-------------|------------|
| 3 | Listing detail screen (price, location, images, seller info) | M1 |
| 3 | Search screen (full-text query, results list, pagination) | M1 |
| 4 | Create listing form (product/service/property, validation) | M1 |
| 4 | My listings screen (seller's own listings, status badges) | M1 |

**Gate:** User can browse, search, and create listings end-to-end. ✅

### M3 — Negotiation & Contact Reveal ✅

| Week | Deliverable | Depends On |
|------|-------------|------------|
| 5 | Negotiation thread (offer history, status timeline) | M2 |
| 5 | Submit offer, accept, reject (buyer + seller roles) | M2 |
| 6 | Request contact reveal (buyer) | M3 week 5 |
| 6 | Approve contact reveal (seller) | M3 week 5 |
| 6 | Polling-based status updates (no push in V1) | M3 week 5 |

**Gate:** Complete negotiation lifecycle works on both platforms. ✅

### M4 — AI Agent Integration ✅

| Week | Deliverable | Depends On |
|------|-------------|------------|
| 7 | Agent chat UI (message list, input bar, typing state) | M3 |
| 7 | Tauri commands for agent endpoints | M3 |
| 8 | "Find laptops under $1000" style natural-language queries | M4 week 7 |
| 8 | Agent-driven negotiation suggestions | M4 week 7 |

**Gate:** User can search and negotiate via natural language through the agent. ✅

### M5 — Production Readiness (Weeks 9–10 — **not started**)

| Week | Deliverable | Depends On |
|------|-------------|------------|
| 9 | Error boundaries, offline state, retry logic | M3 |
| 9 | Push notifications (Tauri notification plugin) | M4 |
| 10 | CI/CD: GitHub Actions builds signed APK + IPA | M5 week 9 |
| 10 | App icons, splash screen, store metadata | M5 week 9 |
| 10 | Performance audit (bundle size, startup time, API latency) | M5 week 9 |

**Gate:** `cargo tauri build` produces store-ready signed binaries.

---

## Dependencies

```
M1 (Foundation)
 └─ M2 (Listing Flow)
     └─ M3 (Negotiation)
         ├─ M4 (AI Agent)
         └─ M5 (Production Polish)
```

M4 and M5 can run in parallel after M3.

---

## Backend Dependencies

The mobile app depends on these backend features, all already implemented:

| Backend Feature | Status | Used By |
|----------------|--------|---------|
| `GET /health` | ✅ | M1 — smoke test |
| `GET /v1/listings/{id}` | ✅ | M2 — detail screen |
| `POST /v1/listings` | ✅ | M2 — create listing |
| `GET /v1/listings/search` | ✅ | M2 — search |
| `POST /v1/negotiations` | ✅ | M3 — open negotiation |
| `POST /v1/negotiations/{id}/offers` | ✅ | M3 — submit offer |
| `POST /v1/negotiations/{id}/accept` | ✅ | M3 — accept |
| `POST /v1/negotiations/{id}/reject` | ✅ | M3 — reject |
| `GET /v1/negotiations/{id}` | ✅ | M3 — negotiation status |
| `POST /v1/negotiations/{id}/request-contact-reveal` | ✅ | M3 — request reveal |
| `POST /v1/contact-reveals/{id}/approve` | ✅ | M3 — approve reveal |
| Idempotency key support | ✅ | all write operations |
| `x-marketplace-claims` auth | ✅ | all endpoints |
| Rate limiting (429) | ✅ | M5 — error handling |

---

## Risk Areas

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Tauri v2 mobile plugin gaps (deep linking, push) | Medium | Medium | Fallback to Tauri notification plugin; defer push to M5 to allow time |
| WebView UI not feeling native on iOS | Low | Medium | Use Svelte transitions and gesture handling; audit against native feel in M5 |
| `keyring` crate cross-platform behavior differences | Low | Low | Wrap in `auth/keystore.rs` with platform-specific fallbacks |
| Backend contract changes during mobile development | Medium | High | Pin `api-contract` version; run live-test.ps1 before merging contract changes |
| Xcode/Provisioning complexity for CI | Medium | Low | Dedicate sprint buffer in M5; use `fastlane` match for signing |

---

## Key Decisions

| Decision | Choice | Date | Rationale |
|----------|--------|------|-----------|
| Framework | Tauri v2 | 2026-05-18 | Rust-native mobile shell, shared types with backend |
| Frontend | Svelte 5 | 2026-05-18 | Smallest bundle, zero virtual DOM, compiles away |
| UI approach | WebView (not native widgets) | 2026-05-18 | Single codebase for both platforms, acceptable for API-driven app |
| Auth header | `x-marketplace-claims` | inherited | Same as MCP and HTTP transports |
| Event model | Polling (no push) | inherited | V1 constraint; push deferred to V2 |
| Shared types | `oz-market-api-contract` crate | inherited | No codegen, zero drift, single source of truth |
| Build target | Android 7+ / iOS 14+ | 2026-05-18 | Tauri v2 defaults, covers ~98% of devices |

---

## Success Criteria (Mobile App)

- [x] `cargo check`, `cargo clippy`, `npm run build` all pass
- [x] Binary compiles and runs without crashing (`marketplace-mobile.exe`)
- [x] Health check via `GET /health`
- [x] Login/logout with keychain-stored claims
- [x] Backend URL configurable via settings screen
- [x] Search listings with pagination
- [x] Listing detail view with price, status, type, description, seller
- [x] Create listing with type selection and form validation
- [x] My Listings screen (owner filtered)
- [x] Open negotiation with initial offer from listing detail
- [x] Negotiation thread with offer history, polling-based updates
- [x] Submit counter-offers, accept, reject
- [x] Request contact reveal
- [x] Approve contact reveal (seller)
- [ ] `cargo tauri android build` produces a signed APK < 5MB
- [ ] `cargo tauri ios build` produces a signed IPA < 10MB
- [x] Agent chat returns results from live backend
- [ ] App passes `docs/specs/openapi.yaml` contract validation
