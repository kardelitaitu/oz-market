# Validation Checklist - Refresh Token Rotation and JWT Blacklist

This checklist is used to confirm the completion of Spec 0027:

- [ ] Blacklist lookup checks are integrated with `auth-core` token verification pipelines.
- [ ] JWT revocation on logout adds token signatures to Redis with correct TTL bounds.
- [ ] Refresh token endpoint yields access + refresh token pairs and invalidates used credentials.
- [ ] Submitting a used refresh token triggers session revocation for all associated user nodes.
