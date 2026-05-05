# Android Identity Contract

## Goal

Describe how the Android app maps user identity into backend seller and agent identity.

## Rules

- user login happens first
- app resolves the user to `seller_account_id`
- app uses short-lived agent tokens for app-scoped actions
- app does not invent alternate identity fields

## Fields To Preserve

- `seller_account_id`
- `agent_credential_id`
- short-lived session token
- `idempotency_key` for replay-sensitive writes

## Notes

- keep this aligned with `docs/whitepaper/11-identity-authz.md`
- keep Android and iOS identity rules identical
