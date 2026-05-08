//! AI Prompt Caching Service
//!
//! Provides caching for AI/LLM prompts to reduce costs and improve performance.
//! Uses Moka cache (already in the project) for in-memory caching.

use moka::sync::Cache;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tracing::{debug, info};

/// Cached AI response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    /// The content of the AI response
    pub content: String,
    /// Model used for the response (e.g., "gpt-4", "openrouter/...")
    pub model: String,
    /// Hash of the prompt (system + user + model) used as cache key
    pub prompt_hash: String,
    /// Timestamp when cached (simplified to string for now)
    pub cached_at: String,
}

/// AI Prompt Cache Service
///
/// Uses Moka cache for in-memory storage with TTL expiration.
/// Designed to be provider-agnostic (works with any AI service).
#[derive(Clone)]
pub struct AiPromptCache {
    cache: Cache<String, CachedResponse>,
    enabled: bool,
}

impl AiPromptCache {
    /// Create a new AI prompt cache
    ///
    /// # Arguments
    /// * `enabled` - Whether caching is enabled
    /// * `max_capacity` - Maximum number of entries in cache
    ///
    /// # Example
    /// ```rust
    /// use marketplace_server::services::ai_cache::AiPromptCache;
    /// let cache = AiPromptCache::new(true, 1000);
    /// ```
    pub fn new(enabled: bool, max_capacity: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(3600)) // 1 hour TTL
            .build();

        info!(
            "AI Prompt Cache initialized: enabled={}, max_capacity={}",
            enabled, max_capacity
        );

        Self { cache, enabled }
    }

    /// Generate a hash for a prompt (used as cache key)
    ///
    /// Combines system_prompt, user_prompt, and model into a single hash.
    fn hash_prompt(system_prompt: &str, user_prompt: &str, model: &str) -> String {
        let mut hasher = DefaultHasher::new();
        system_prompt.hash(&mut hasher);
        user_prompt.hash(&mut hasher);
        model.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Try to get a cached response
    ///
    /// Returns `Some(CachedResponse)` if found, `None` if not in cache or cache disabled.
    pub fn get_cached(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        model: &str,
    ) -> Option<CachedResponse> {
        if !self.enabled {
            return None;
        }

        let key = Self::hash_prompt(system_prompt, user_prompt, model);

        match self.cache.get(&key) {
            Some(response) => {
                debug!(
                    "AI cache HIT for prompt hash: {} (model: {})",
                    &key[..8],
                    model
                );
                Some(response)
            }
            None => {
                debug!(
                    "AI cache MISS for prompt hash: {} (model: {})",
                    &key[..8],
                    model
                );
                None
            }
        }
    }

    /// Cache an AI response
    ///
    /// Stores the response in cache if caching is enabled.
    pub fn cache_response(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        model: &str,
        content: &str,
    ) {
        if !self.enabled {
            return;
        }

        let key = Self::hash_prompt(system_prompt, user_prompt, model);
        let cached = CachedResponse {
            content: content.to_string(),
            model: model.to_string(),
            prompt_hash: key.clone(),
            cached_at: "now".to_string(), // Simplified
        };

        self.cache.insert(key.clone(), cached);

        debug!(
            "AI response cached: prompt hash={} (model: {})",
            &key[..8],
            model
        );
    }

    /// Get cache statistics
    ///
    /// Returns (entry_count, weighted_size) where weighted_size is approximate memory usage.
    pub fn stats(&self) -> (u64, u64) {
        (self.cache.entry_count(), self.cache.weighted_size())
    }

    /// Clear the cache (useful for testing)
    #[cfg(test)]
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let cache = AiPromptCache::new(true, 100);

        // Cache a response
        cache.cache_response(
            "You are a helpful assistant",
            "What is 2+2?",
            "gpt-3.5-turbo",
            "4",
        );

        // Should get a hit
        let cached = cache.get_cached(
            "You are a helpful assistant",
            "What is 2+2?",
            "gpt-3.5-turbo",
        );

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "4");
    }

    #[test]
    fn test_cache_miss() {
        let cache = AiPromptCache::new(true, 100);

        // Don't cache anything

        // Should be a miss
        let cached = cache.get_cached(
            "You are a helpful assistant",
            "What is 2+2?",
            "gpt-3.5-turbo",
        );

        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_disabled() {
        let cache = AiPromptCache::new(false, 100);

        // Cache a response
        cache.cache_response(
            "You are a helpful assistant",
            "What is 2+2?",
            "gpt-3.5-turbo",
            "4",
        );

        // Should still be None because cache is disabled
        let cached = cache.get_cached(
            "You are a helpful assistant",
            "What is 2+2?",
            "gpt-3.5-turbo",
        );

        assert!(cached.is_none());
    }
}
