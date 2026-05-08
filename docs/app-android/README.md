# Android App Documentation

This folder contains Android client documentation.

## Intended Contents

- app architecture
- screen map
- authentication flow
- user-created AI agent flow using `openrouter/free`
- API integration notes
- push notification strategy
- offline caching approach

## Current Status (Updated 2026-05-08)

The server is **production-ready**! The Android app should integrate with:

| Backend Feature | Status | API Endpoint |
|-----------------|--------|--------------|
| **Listings** | ✅ Complete | `POST /api/listings`, `GET /api/listings/{id}`, `POST /api/listings/search` |
| **Reviews** | ✅ Complete | `POST /api/reviews`, `GET /api/listings/{id}/reviews` |
| **Negotiations** | ✅ Complete | `POST /api/negotiations`, `POST /api/negotiations/{id}/offers` |
| **Contact Reveals** | ✅ Complete | `POST /api/contact-reveals`, `POST /api/contact-reveals/{id}/approve` |
| **Admin** | ✅ Complete | Archive listings, release negotiations, trust levels |
| **Performance** | ✅ **42k+ ops/s** | 8.2× above 5k target! |
| **OpenAPI Spec** | ✅ **20+ endpoints** | Full documentation at `/docs` |

## API Integration Notes

### Base URL
```
Production: https://api.oz-market.com
Development: http://10.0.2.2:3003  (Android emulator to localhost)
```

### Authentication
The app should use **JWT tokens** with the following claims structure:
```json
{
  "sub": "user_id",
  "roles": ["buyer_searcher", "buyer_negotiator"],
  "scopes": ["listings:search", "negotiations:write"]
}
```

See `../whitepaper/11-identity-authz.md` for full auth model.

### Key API Patterns

#### 1. Search Listings (Most Common)
```kotlin
// POST /api/listings/search
val request = SearchRequest(
    query = "laptop",
    min_price = 500,
    max_price = 1500,
    sort_by = "price_lowest",
    page = 1,
    page_size = 20
)
```

#### 2. Create Listing (Sellers)
```kotlin
// POST /api/listings
val request = CreateListingRequest(
    title = "ThinkPad X1 Carbon",
    description = "Excellent condition...",
    price = 800.00,
    currency = "USD",
    category = "electronics",
    location = "New York, NY"
)
```

#### 3. Negotiate (Buyers)
```kotlin
// POST /api/negotiations
val request = OpenNegotiationRequest(
    listing_id = "list_123",
    initial_offer = 700.00,
    message = "Would you take $700?"
)
```

## App Architecture Recommendations

### Pattern: MVVM + Repository
```
app/
├── ui/           # Views (Activities, Fragments)
├── viewmodels/   # ViewModels (state management)
├── repository/   # Repository (API calls, caching)
├── model/        # Data models (matching OpenAPI spec)
├── agent/        # AI agent logic (openrouter/free)
└── utils/        # Helpers (auth, errors, etc.)
```

### Key Libraries
- **Retrofit** (HTTP client with OpenAPI-generated models)
- **Coroutines** (async/await)
- **Flow** (reactive streams)
- **Hilt** (dependency injection)
- **Compose** (modern UI, or XML if preferred)

## User-Created AI Agent Flow

The app should allow users to configure their own AI agent using `openrouter/free`:

### 1. Agent Settings Screen
```
User goes to Settings → AI Agent
- Select model: "openrouter/free" (default)
- Configure agent behavior (buyer/seller)
- Set search preferences
- Enable/disable auto-negotiation
```

### 2. Agent Integration
```kotlin
class AiAgentRepository {
    suspend fun searchWithAgent(query: String): List<Listing> {
        val response = openRouterApi.chatCompletion(
            model = "openrouter/free",
            messages = listOf(
                Message("system", "You are a buyer's agent..."),
                Message("user", "Search for: $query")
            )
        )
        // Parse agent response and call our API
        return api.searchListings(parseAgentRequest(response))
    }
}
```

## Next Docs To Add

1. ✅ **Screen and navigation flow** (see `first-user-flow.md`)
2. 🔜 **Mobile auth flow** (JWT + refresh tokens)
3. 🔜 **App-agent lifecycle and settings**
4. 🔜 **Push notification strategy** (negotiation updates, contact reveals)
5. 🔜 **Offline caching approach** (Room database for listings)
6. 🔜 **Image handling** (we don't support images yet per spec!)

## Open Questions for Mobile

1. **How to handle contact reveal UX?**
   - Show phone number after approval?
   - In-app messaging instead?

2. **How to display negotiations?**
   - Chat-like interface?
   - List with status updates?

3. **How to handle AI agent errors?**
   - Retry logic?
   - Fallback to manual search?

---

**The backend is ready!** Start building the Android app. 🚀

See `first-user-flow.md` for the initial user journey.
See `../specs/openapi.yaml` for complete API specification.
See `../whitepaper/10-api-contract.md` for the canonical contract.
