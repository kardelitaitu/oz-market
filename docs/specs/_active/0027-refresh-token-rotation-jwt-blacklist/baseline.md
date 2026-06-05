# Baseline - Refresh Token Rotation and JWT Blacklist

## Current State

As of starting Phase 4:
- The `auth-core` crate handles stateless JWT validation without any blacklist checks.
- Revocation of tokens is impossible before their expiration time.
- No refresh token endpoint or rotation logic exists; clients must re-authenticate with credentials when tokens expire.
