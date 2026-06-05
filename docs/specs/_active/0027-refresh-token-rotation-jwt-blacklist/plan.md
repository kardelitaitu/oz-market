# Plan - Refresh Token Rotation and JWT Blacklist

## Implementation Steps

1. **Blacklist Integration**:
   - Create `backend/crates/auth-core/src/blacklist.rs`.
   - Implement `JwtBlacklist` service querying Redis cache.
   - Update auth middleware to query blacklist using the token's `jti` claim.

2. **Refresh Token Rotation Implementation**:
   - Create `backend/crates/auth-core/src/token_rotation.rs`.
   - Declare refresh token generator.
   - Store generated refresh tokens in Redis.

3. **Refresh Route & Handler**:
   - Create `/api/v1/auth/refresh` controller in Actix server.
   - Process tokens: check validity, invalidate old refresh token, generate and return new pair.
   - Implement breach recovery (revoke all sessions) if a used refresh token is re-submitted.
