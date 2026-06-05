# AI Prompt Cache

## Overview

The AI Prompt Cache provides caching for AI/LLM prompts to reduce API costs and improve response times. It uses the Moka cache (already integrated in the project) for in-memory caching with TTL-based expiration.

## Implementation

- **Location**: `backend/server/src/services/ai_cache.rs`
- **Cache Engine**: Moka (same as used for listing caching in Phase 1)
- **Cache Key**: SHA-256 hash of `(system_prompt + user_prompt + model)`
- **TTL**: 1 hour (3,600 seconds) default
- **Capacity**: Configurable via `max_capacity` parameter

## API

### `AiPromptCache::new(enabled: bool, max_capacity: u64) -> Self`

Creates a new prompt cache instance.

```rust
let cache = AiPromptCache::new(true, 1000);
```

### `get_cached(system_prompt, user_prompt, model) -> Option<CachedResponse>`

Retrieves a cached response if it exists and caching is enabled.

```rust
if let Some(cached) = cache.get_cached("You are a helpful assistant", "What is 2+2?", "gpt-4") {
    println!("Cache hit! Response: {}", cached.content);
}
```

### `cache_response(system_prompt, user_prompt, model, content)`

Stores an AI response in the cache.

```rust
cache.cache_response(
    "You are a helpful assistant",
    "What is 2+2?",
    "gpt-4",
    "4"
);
```

### `stats() -> (u64, u64)`

Returns cache statistics: `(entry_count, weighted_size)`.

```rust
let (count, size) = cache.stats();
println!("Cache entries: {}, size: {}", count, size);
```

## `CachedResponse` Struct

```rust
pub struct CachedResponse {
    pub content: String,        // AI response text
    pub model: String,          // Model used (e.g., "gpt-4")
    pub prompt_hash: String,    // Hash of the prompt
    pub cached_at: String,      // Timestamp (simplified)
}
```

## Usage with AI Providers

The cache is provider-agnostic. Example integration with OpenRouter (as per whitepaper):

```rust
async fn get_ai_response(cache: &AiPromptCache, system: &str, user: &str, model: &str) -> String {
    // Check cache first
    if let Some(cached) = cache.get_cached(system, user, model) {
        return cached.content;
    }
    
    // Call AI provider (e.g., OpenRouter)
    let response = call_openrouter(system, user, model).await;
    
    // Cache the response
    cache.cache_response(system, user, model, &response);
    
    response
}
```

## Configuration

- **Enabled/Disabled**: Pass `enabled` flag to constructor
- **Capacity**: Max number of cache entries (eviction policy: LRU-like)
- **TTL**: Currently hardcoded to 1 hour; could be made configurable

## Testing

The module includes 3 unit tests:
- `test_cache_hit` - verifies cached responses are retrieved
- `test_cache_miss` - verifies missing prompts return `None`
- `test_cache_disabled` - verifies disabled cache doesn't store/retrieve

Run tests:
```bash
cd backend && cargo test --package oz-market-server ai_cache
```

## Future Enhancements

- Add cost tracking (tokens saved = cost saved)
- Cache warming for common prompts
- Persistent cache (Redis) for multi-instance deployments
- Metrics endpoint to expose cache hit/miss rates

## Integration with Mobile Apps

Per the whitepaper, mobile users can create their own free AI agent. The server-managed AI cache allows:
- Users without API keys use server's cache + server API key
- Users with own keys can still benefit from caching (if they opt-in)
- Reduces costs for free-tier usage

## Related Files

- `backend/server/src/services/ai_cache.rs` - Implementation
- `backend/server/src/services/mod.rs` - Module declaration
- `docs/specs/openapi.yaml` - API reference (separate, for HTTP endpoints)
- Whitepaper section on AI integration
