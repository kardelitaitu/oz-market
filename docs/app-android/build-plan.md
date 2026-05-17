# Android Build Plan

## Overview

Build the Android app from scaffold to production-ready client aligned with the frozen V1 API contract at `docs/specs/openapi.yaml`.

**Architecture:** MVVM + Repository (as documented in `docs/app-android/README.md`)

**Key constraints:**
- use the same backend contract as HTTP and MCP — no Android-only payload variants
- keep authz and abuse controls server-side
- polling-first event model (no mobile event stream in V1)
- idempotency keys for all replay-sensitive writes

---

## Phase 1 — Project Scaffold

**Goal:** A compilable Android project with build configuration and package structure.

### Files to create

```
mobile/app-android/
├── build.gradle.kts              (project-level)
├── settings.gradle.kts
├── gradle.properties
├── gradle/
│   └── libs.versions.toml        (version catalog)
├── app/
│   ├── build.gradle.kts          (module-level)
│   └── src/
│       └── main/
│           ├── AndroidManifest.xml
│           └── java/com/marketplace/android/
│               ├── MarketplaceApp.kt          (Application class)
│               ├── MainActivity.kt
│               ├── di/                         (Hilt modules)
│               ├── data/
│               │   ├── api/                    (Retrofit interfaces)
│               │   ├── model/                  (DTOs from OpenAPI)
│               │   └── repository/             (Repository implementations)
│               ├── domain/
│               │   └── model/                  (domain models)
│               ├── ui/
│               │   ├── listing/
│               │   ├── search/
│               │   ├── negotiation/
│               │   └── settings/
│               └── agent/                      (openrouter/free integration)
```

### Key dependencies
- Retrofit + OkHttp + Moshi/kotlinx-serialization
- Hilt (DI)
- Kotlin Coroutines + Flow
- Jetpack Compose (UI)
- Navigation Compose
- Room (V2, Phase 6)
- OpenAPI Generator Gradle plugin

### Acceptance
- `./gradlew assembleDebug` succeeds
- app launches on emulator showing a shell screen

---

## Phase 2 — API Client & Data Layer

**Goal:** Generated Retrofit client from the frozen OpenAPI spec, repository pattern with error handling.

### Steps
1. Configure `org.openapi.generator` Gradle plugin
2. Generate Retrofit interfaces + data classes from `docs/specs/openapi.yaml`
3. Wire OkHttp client with:
   - auth interceptor (injects JWT from session)
   - logging interceptor (debug builds)
   - timeout and retry configuration
4. Implement base repository with:
   - `Result<T>` wrapper for success/error/loading states
   - idempotency key generation and retry logic
   - error mapping (409 conflict, 403 forbidden, 404 not found, 429 rate limit, 5xx)
5. Write repository interfaces for:
   - `ListingRepository` (create, get, search)
   - `NegotiationRepository` (open, submit offer, accept, reject, get status)
   - `ContactRevealRepository` (request, approve, poll)
   - `AuthRepository` (sign in, refresh token)

### Key API patterns
```kotlin
// Search
suspend fun search(request: SearchRequest): Result<SearchResponse>

// Create listing with idempotency
suspend fun createListing(
    payload: CreateListingRequest,
    idempotencyKey: String = generateKey()
): Result<CreateListingResponse>

// Negotiation lifecycle
suspend fun openNegotiation(request: OpenNegotiationRequest): Result<NegotiationResponse>
suspend fun submitOffer(negotiationId: String, request: SubmitOfferRequest): Result<NegotiationResponse>
suspend fun acceptNegotiation(negotiationId: String, idempotencyKey: String): Result<NegotiationResponse>
suspend fun rejectNegotiation(negotiationId: String, idempotencyKey: String): Result<NegotiationResponse>
```

### Acceptance
- generated API client compiles with all endpoints
- repository unit tests pass with mock HTTP
- idempotency replay returns cached response instead of duplicate writes

---

## Phase 3 — Auth & Session

**Goal:** User sign-in, JWT acquisition, session management, and token injection into all API calls.

### Steps
1. Build sign-in screen (email/password or OAuth)
2. Implement `AuthRepository`:
   - `signIn(email, password): Result<Session>`
   - `refreshToken(): Result<Session>`
   - `signOut()`
3. Session manager:
   - store token in EncryptedSharedPreferences
   - expose `Claims` for role/scope checks
   - auto-refresh on 401 responses via OkHttp interceptor
4. Wire auth interceptor to inject `Authorization: Bearer <token>` header
5. Navigation guard — redirect unauthenticated users to sign-in

### Contract alignment
- Claims shape matches `docs/whitepaper/11-identity-authz.md`
- Session tokens are short-lived, separate from seller identity
- Key fields: `seller_account_id`, `buyer_agent_id`, `roles`, `scopes`

### Acceptance
- sign-in flow completes end-to-end with mock backend
- expired token triggers refresh automatically
- sign-out clears session and redirects to sign-in
- 401 from any API call triggers re-auth

---

## Phase 4 — Core UI Screens

**Goal:** First user flows implemented — seller create listing, buyer search + detail, negotiation lifecycle, settings.

### Screens

| Screen | Flow | Priority |
|--------|------|----------|
| ListingCreateScreen | Seller fills form → submit → show result | P0 |
| SearchScreen | Query + filters → results list | P0 |
| ListingDetailScreen | Full listing view → start negotiation | P0 |
| NegotiationScreen | Offer history → submit → accept/reject → contact reveal | P0 |
| SettingsScreen | AI agent config, account info | P1 |
| MyListingsScreen | Seller views/manages own listings | P1 |

### UI patterns
- Compose state hoisting with `StateFlow` in ViewModels
- Loading/success/error states for every screen
- Idempotency key displayed for debugging
- Pull-to-refresh on search and listing detail

### Acceptance
- seller creates a listing end-to-end
- buyer searches, views detail, opens negotiation, submits offer
- seller views negotiation, accepts/rejects
- contact reveal flow works
- error states displayed (network error, 403, 404, 409, 429)

---

## Phase 5 — AI Agent Integration

**Goal:** User-configurable `openrouter/free` agent for automated search and negotiation.

### Steps
1. Agent settings screen:
   - enable/disable agent
   - configure buyer/seller role
   - set search preferences and price limits
   - enable/disable auto-negotiation
2. Agent service layer:
   - wraps `openrouter/free` chat completion API
   - prompt engineering for listing search and negotiation
   - response parser maps agent output to API calls
3. Agent execution flow:
   - user triggers agent action
   - agent calls `openrouter/free` with structured prompt
   - agent parses response → calls marketplace API
   - agent reports result to user

### Contract rules
- agent only uses the same backend contract
- agent setup is app-scoped, not a backend trust anchor
- provider choice is user-side only

### Acceptance
- agent searches listings with natural language query
- agent opens negotiation with price suggestion
- agent settings are persisted and restored

---

## Phase 6 — Polish & Production Readiness

**Goal:** Offline resilience, push notifications, image support, and performance.

### Items
1. **Room database** — offline cache for listings, search results
   - cache strategy: cache-first, network refresh
   - eviction: TTL-based
2. **Push notifications** — FCM integration for negotiation updates and contact reveals
3. **Image handling** — Coil for listing picture loading, camera/gallery picker for create listing
4. **Error analytics** — crash reporting, network error tracking
5. **Progressive enhancement** — pagination, search debounce, optimistic UI for writes
6. **Accessibility** — content descriptions, talkback support, minimum touch targets

### Acceptance
- app works offline with cached listings
- push notifications arrive for negotiation events
- images load with placeholders and error states
- app passes basic accessibility audit

---

## Dependency Graph

```
Phase 1 (Scaffold)
   └── Phase 2 (API Client)
          └── Phase 3 (Auth)
                 └── Phase 4 (UI Screens)
                        └── Phase 5 (AI Agent)
                               └── Phase 6 (Polish)
```

Each phase depends on the previous. Phase 2 and 3 can partially overlap.

---

## Open Questions

1. **OAuth provider** — which identity provider for sign-in? (Google, email/password self-managed, or custom)
2. **Push notification delivery** — does the backend have a notification endpoint, or is this entirely app-side FCM?
3. **Image upload endpoint** — does the backend accept image uploads, or only URLs? Current spec says `picture_urls` only.
4. **Min SDK** — API 26 (Android 8.0) or higher? Recommended: API 26 for ~95% coverage.
5. **Test strategy** — unit tests with mocked repos, or instrumented tests against a test backend?
