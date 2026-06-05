# Decisions - Refresh Token Rotation and JWT Blacklist

## Architecture Decisions

### 1. Redis for Transient Revocations
- **Decision**: Blacklisted token identifiers (`jti`) and active refresh tokens will be stored in Redis instead of Postgres.
- **Rationale**: Telemetry check calls run on every request. Querying Redis is extremely fast (sub-millisecond) and keys expire natively via TTL, avoiding persistent log accumulations.

### 2. Immediate Session Revocation on RTR Reuse
- **Decision**: If a client attempts to refresh a session using a refresh token that has already been flagged as used/deleted, invalidate all active sessions for that user account immediately.
- **Rationale**: Token reuse indicates potential token theft. revoking all active sessions forces re-authentication, isolating the compromise.
