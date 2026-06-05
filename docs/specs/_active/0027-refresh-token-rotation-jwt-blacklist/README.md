---
id: 0027-refresh-token-rotation-jwt-blacklist
title: Refresh Token Rotation and JWT Blacklist
status: active
owner: backend-team
implementer: agent
priority: P2
---

# Refresh Token Rotation and JWT Blacklist

Status: `active`
Implementer: `agent`

## Summary

This specification defines the implementation of a Refresh Token Rotation (RTR) scheme and a dynamic JWT blacklist. It ensures that stateless access tokens can be blacklisted instantly upon user logout, and refresh tokens are strictly single-use to prevent reuse attacks.

## Scope

### In Scope
- Implementing `TokenBlacklist` storing signature hashes in Redis.
- Intercepting auth middleware checks to reject blacklisted tokens.
- Creating the `/api/v1/auth/refresh` endpoint executing Token Rotation.
- Storing active refresh tokens in Redis with single-use markers.

### Out of Scope
- Storing blacklist histories permanently (keys expire natively via Redis TTL).

## Proposed Direction
1. JWT Blacklist:
   - On logout, compute token remaining validity duration.
   - Insert key `blacklist:token:{jti}` with TTL equal to the validity duration.
2. Refresh Token Rotation:
   - A refresh token is stored in Redis.
   - When used, verify it is valid, delete it from Redis immediately, and return a new access + refresh token pair.
   - If a deleted (used) refresh token is submitted again, immediately invalidate all active tokens for that user account (Reuse Detection).
