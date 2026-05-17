# iOS App Documentation

This folder contains iOS client documentation.

## Intended Contents

- app architecture
- screen map
- authentication flow
- user-created AI agent flow using `openrouter/free`
- API integration notes
- push notification strategy (APNs)
- offline caching approach (Core Data)

## Current Status (Updated 2026-05-08)

The server is **production-ready**! The iOS app should integrate with:

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
```swift
let baseURL = "https://api.oz-market.com"
// Development:
let baseURL = "http://localhost:3003"  // Use computer's IP on simulator
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
```swift
// POST /api/listings/search
struct SearchRequest: Codable {
    let query: String?
    let minPrice: Double?
    let maxPrice: Double?
    let sortBy: String?
    let page: Int?
    let pageSize: Int?
    
    enum CodingKeys: String, CodingKey {
        case query, minPrice = "min_price", maxPrice = "max_price"
        case sortBy = "sort_by", page, pageSize = "page_size"
    }
}
```

#### 2. Create Listing (Sellers)
```swift
// POST /api/listings
struct CreateListingRequest: Codable {
    let title: String
    let description: String
    let price: Double
    let currency: String
    let category: String
    let location: String
}
```

#### 3. Negotiate (Buyers)
```swift
// POST /api/negotiations
struct OpenNegotiationRequest: Codable {
    let listingId: String
    let initialOffer: Double
    let message: String?
    
    enum CodingKeys: String, CodingKey {
        case listingId = "listing_id", initialOffer = "initial_offer", message
    }
}
```

## App Architecture Recommendations

### Pattern: MVVM + Combine
```
OzMarket/
├── Views/          # SwiftUI views
├── ViewModels/     # ObservableObject classes
├── Models/         # Codable structs (matching OpenAPI spec)
├── Services/       # API service, caching
├── Agent/          # AI agent logic (openrouter/free)
└── Utilities/      # Extensions, helpers
```

### Key Libraries
- **Alamofire** (HTTP client, or URLSession + Combine)
- **Combine** (reactive programming)
- **SwiftUI** (modern UI framework)
- **KeychainAccess** (secure token storage)
- **Core Data** (optional offline caching)

## User-Created AI Agent Flow

The app should allow users to configure their own AI agent using `openrouter/free`:

### 1. Agent Settings Screen (SwiftUI)
```swift
struct AgentSettingsView: View {
    @State private var selectedModel = "openrouter/free"
    @State private var isAgentEnabled = true
    
    var body: some View {
        Form {
            Section("AI Agent") {
                Toggle("Enable AI Agent", isOn: $isAgentEnabled)
                Picker("Model", selection: $selectedModel) {
                    Text("openrouter/free").tag("openrouter/free")
                }
            }
        }
    }
}
```

### 2. Agent Integration
```swift
class AiAgentService {
    func searchWithAgent(query: String) async throws -> [Listing] {
        let request = ChatCompletionRequest(
            model: "openrouter/free",
            messages: [
                Message(role: "system", content: "You are a buyer's agent..."),
                Message(role: "user", content: "Search for: \(query)")
            ]
        )
        
        let response = try await openRouterApi.chatCompletion(request)
        // Parse agent response and call our API
        let searchRequest = parseAgentRequest(response)
        return try await apiService.searchListings(searchRequest)
    }
}
```

## Next Docs To Add

1. ✅ **Screen and navigation flow** (see `first-user-flow.md`)
2. ✅ **Full build plan** (see `build-plan.md`)
3. 🔜 **Mobile auth flow** (JWT + Keychain, refresh tokens)
4. 🔜 **App-agent lifecycle and settings**
5. 🔜 **Push notification strategy** (APNs integration)
6. 🔜 **Offline caching approach** (Core Data for listings)
7. 🔜 **Image handling** (we don't support images yet per spec!)

## Open Questions for Mobile

1. **How to handle contact reveal UX?**
   - Show phone number after approval?
   - In-app messaging instead?

2. **How to display negotiations?**
   - iMessage-like interface?
   - List with status updates?

3. **How to handle AI agent errors?**
   - Retry with exponential backoff?
   - Fallback to manual search?

4. **iOS-specific: How to handle background fetch?**
   - BGTaskScheduler for negotiation updates?
   - Silent push notifications?

---

**The backend is ready!** Start building the iOS app. 🚀

See `first-user-flow.md` for the initial user journey.
See `../specs/openapi.yaml` for complete API specification.
See `../whitepaper/10-api-contract.md` for the canonical contract.
