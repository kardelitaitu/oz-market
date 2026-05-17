# iOS Build Plan

## Overview

Build the iOS app from scaffold to production-ready client aligned with the frozen V1 API contract at `docs/specs/openapi.yaml`.

**Architecture:** MVVM + async-await (as documented in `docs/app-ios/README.md`)

**Key constraints:**
- use the same backend contract as HTTP and MCP — no iOS-only payload variants
- keep authz and abuse controls server-side
- polling-first event model (no mobile event stream in V1)
- idempotency keys for all replay-sensitive writes
- keep Android and iOS identity, polling, and payload rules identical

---

## Phase 1 — Project Scaffold

**Goal:** A compilable Xcode project with Swift package dependencies and source structure.

### Files to create

```
mobile/app-ios/
├── OzMarket.xcodeproj/
├── OzMarket/
│   ├── OzMarketApp.swift              (App entry point)
│   ├── Info.plist
│   ├── DI/
│   │   └── DependencyContainer.swift  (manual DI or factory)
│   ├── Data/
│   │   ├── API/
│   │   │   ├── APIClient.swift            (URLSession-based HTTP client)
│   │   │   ├── Endpoints.swift            (enum-based route definitions)
│   │   │   └── DTOs.swift                 (Codable structs from OpenAPI)
│   │   └── Repository/
│   │       ├── ListingRepository.swift
│   │       ├── NegotiationRepository.swift
│   │       ├── ContactRevealRepository.swift
│   │       └── AuthRepository.swift
│   ├── Domain/
│   │   └── Models/
│   ├── UI/
│   │   ├── Listing/
│   │   ├── Search/
│   │   ├── Negotiation/
│   │   └── Settings/
│   ├── Agent/                          (openrouter/free integration)
│   └── Utilities/
│       ├── KeychainManager.swift
│       └── IdempotencyKeyGenerator.swift
├── OzMarketTests/
└── Package.swift                       (or SPM dependencies)
```

### Key dependencies (Swift Package Manager)
- No Alamofire — use `URLSession` + async-await (native, lightweight)
- `swift-openapi-generator` for OpenAPI client code generation
- KeychainAccess (secure token storage)
- Optionally: Firebase Cloud Messaging (Phase 6)

### Acceptance
- `xcodebuild` succeeds
- app launches on simulator showing a shell screen

---

## Phase 2 — API Client & Data Layer

**Goal:** OpenAPI-generated HTTP client, repository pattern with error handling.

### Steps
1. Configure `swift-openapi-generator` with `docs/specs/openapi.yaml`
2. Generate `APIClient` + `Codable` DTOs
3. Implement `APIClient` with:
   - auth token injection via `URLRequest` interceptor
   - configurable base URL (production vs local dev)
   - timeout configuration
   - `Result<T, APIError>` return type
4. Define `APIError` enum:
   - `unauthorized`, `forbidden`, `notFound`, `conflict`, `rateLimited`, `internalError`, `networkError`, `decodingError`
5. Write repository protocols + implementations:
   - `ListingRepositoryProtocol` (create, get, search)
   - `NegotiationRepositoryProtocol` (open, submit offer, accept, reject, get status)
   - `ContactRevealRepositoryProtocol` (request, approve, poll)
   - `AuthRepositoryProtocol` (sign in, refresh token)

### Key API patterns
```swift
// Search
func search(_ request: SearchRequest) async throws -> Result<SearchResponse, APIError>

// Create listing with idempotency
func createListing(
    _ payload: CreateListingRequest,
    idempotencyKey: String = IdempotencyKeyGenerator.new()
) async throws -> Result<CreateListingResponse, APIError>

// Negotiation lifecycle
func openNegotiation(_ request: OpenNegotiationRequest) async throws -> Result<NegotiationResponse, APIError>
func submitOffer(_ negotiationId: String, _ request: SubmitOfferRequest) async throws -> Result<NegotiationResponse, APIError>
func acceptNegotiation(_ negotiationId: String, idempotencyKey: String) async throws -> Result<NegotiationResponse, APIError>
func rejectNegotiation(_ negotiationId: String, idempotencyKey: String) async throws -> Result<NegotiationResponse, APIError>
```

### Acceptance
- generated client compiles with all endpoints
- repository unit tests pass with `URLProtocol` mock
- idempotency guard works on create and negotiate writes

---

## Phase 3 — Auth & Session

**Goal:** User sign-in, JWT acquisition, secure token storage, and auth injection.

### Steps
1. Build sign-in view (email/password or Sign in with Apple)
2. Implement `AuthRepository`:
   - `signIn(email, password) async throws -> Session`
   - `refreshToken() async throws -> Session`
   - `signOut()`
3. Session manager using Keychain:
   - store JWT in Keychain via `KeychainAccess`
   - expose current `Claims` (sub, roles, scopes)
   - auto-refresh on 401 via URLSession delegate
4. Wire `AuthInterceptor` to add `Authorization: Bearer <token>` header
5. Navigation guard — redirect unauthenticated to sign-in

### Contract alignment
- Claims shape matches `docs/whitepaper/11-identity-authz.md`
- Session tokens are short-lived, separate from seller identity
- Key fields: `seller_account_id`, `buyer_agent_id`, `roles`, `scopes`

### Acceptance
- sign-in completes end-to-end with mock backend
- expired token triggers refresh automatically
- sign-out clears Keychain and returns to sign-in
- 401 triggers re-auth

---

## Phase 4 — Core UI Screens

**Goal:** First user flows — seller create listing, buyer search + detail, negotiation lifecycle, settings.

### Screens (SwiftUI)

| Screen | Flow | Priority |
|--------|------|----------|
| CreateListingView | Seller fills form → submit → show result | P0 |
| SearchView | Query + filters → results list | P0 |
| ListingDetailView | Full listing → start negotiation | P0 |
| NegotiationView | Offer history → submit → accept/reject → contact reveal | P0 |
| SettingsView | AI agent config, account info | P1 |
| MyListingsView | Seller manages own listings | P1 |

### UI patterns
- `@ObservableObject` ViewModels with `@Published` state
- `AsyncImage` for listing pictures (Phase 6)
- `enum LoadingState<T>` for loading/success/error/empty
- Pull-to-refresh via `.refreshable { }`
- `ProgressView` for loading states

### Acceptance
- seller creates listing end-to-end
- buyer searches, views detail, opens negotiation, submits offer
- seller views negotiation, accepts/rejects
- contact reveal flow works
- error states displayed (no connection, 403, 404, 409, 429)

---

## Phase 5 — AI Agent Integration

**Goal:** User-configurable `openrouter/free` agent for automated search and negotiation.

### Steps
1. Agent settings view:
   - toggle enable/disable
   - configure buyer/seller role
   - set search preferences and price limits
   - enable/disable auto-negotiation
2. Agent service layer:
   - wraps `openrouter/free` chat completion API via URLSession
   - structured prompts for listing search and negotiation
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
- agent settings persist across app launches

---

## Phase 6 — Polish & Production Readiness

**Goal:** Offline resilience, push notifications, image support, and performance.

### Items
1. **Core Data cache** — offline listing storage
   - cache-first with network refresh
   - TTL-based eviction
2. **Push notifications** — APNs via Firebase Cloud Messaging
   - negotiation updates, contact reveal approvals
3. **Image handling** — `AsyncImage` caching, `PhotosPicker` for create listing
4. **Background tasks** — `BGTaskScheduler` for negotiation state polling
5. **Crash reporting** — Firebase Crashlytics or Sentry
6. **Accessibility** — VoiceOver labels, dynamic type, reduced motion

### Acceptance
- app works offline with cached listings
- push notifications arrive for negotiation events
- images load with placeholders and error states
- app passes basic VoiceOver audit

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

1. **Sign-in provider** — Sign in with Apple, email/password, or both? Backend needs to support the chosen flow.
2. **Push notification delivery** — does backend have a notification endpoint, or is this app-side FCM/APNs only?
3. **Image upload** — does the backend accept image uploads or only URLs? Current spec says `picture_urls` only.
4. **Minimum iOS version** — iOS 16+ recommended for SwiftUI async-await and `.refreshable` support. iOS 17 would allow `@Observable` macro.
5. **Code generation** — use `swift-openapi-generator` at build time, or hand-write Codable structs from the spec? Generator reduces drift risk.
