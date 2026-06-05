# Implementation Notes - Refresh Token Rotation and JWT Blacklist

## JWT Verification Middleware Integration

```rust
use redis::AsyncCommands;

pub async fn is_token_blacklisted(
    redis: &mut redis::aio::ConnectionManager,
    jti: &str,
) -> bool {
    let key = format!("blacklist:token:{}", jti);
    let exists: Result<bool, _> = redis.exists(&key).await;
    exists.unwrap_or(false)
}
```

## Refresh Token Rotation Logic

```rust
use redis::AsyncCommands;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Token reuse detected")]
    ReuseDetected,
    #[error("Database error: {0}")]
    Database(String),
}

pub async fn rotate_refresh_token(
    redis: &mut redis::aio::ConnectionManager,
    submitted_token: &str,
    user_id: &str,
) -> Result<(String, String), AuthError> {
    let key = format!("auth:refresh:{}", submitted_token);
    
    // 1. Check if token exists in Redis
    let exists: bool = redis.exists(&key).await.unwrap_or(false);
    if !exists {
        // Token Reuse detected! Invalidate all refresh tokens for this user
        invalidate_all_sessions(redis, user_id).await;
        return Err(AuthError::ReuseDetected);
    }

    // 2. Invalidate submitted token immediately (single-use)
    let _: () = redis.del(&key).await.unwrap_or(());

    // 3. Generate new pair
    let new_access = generate_access_token(user_id);
    let new_refresh = generate_refresh_token();
    
    // 4. Save new refresh token in Redis with 7-day TTL
    let new_key = format!("auth:refresh:{}", new_refresh);
    let _: () = redis.set_ex(&new_key, user_id, 7 * 24 * 3600).await.unwrap_or(());

    Ok((new_access, new_refresh))
}

async fn invalidate_all_sessions(redis: &mut redis::aio::ConnectionManager, user_id: &str) {
    // Scan and delete all keys associated with this user
}
```
