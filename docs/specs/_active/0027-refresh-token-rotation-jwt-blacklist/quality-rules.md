# Quality Rules - Refresh Token Rotation and JWT Blacklist

- **Bounded Blacklist Key TTL**: Blacklisted token keys must have their Redis TTL set precisely to the remaining duration of the token to prevent memory leaks.
- **Fail-Open Auth Bypass**: If Redis goes down, blacklist validation should fail-open (relying on normal stateless token expiration checks) to maintain uptime.
- **Strict Reuse Detection**: Token rotation handlers must delete the old refresh token *before* returning new ones to ensure concurrent double-submit attempts fail.
