# CI Commands

Run from repo root:

```powershell
./check.ps1
```

Focused checks for this spec:

```powershell
# Verify current frozen negotiation paths
rg -n "/negotiations|request-contact-reveal|contact-reveals/.*/approve" docs/specs/openapi.yaml

# Verify accept/reject and offer_history are not yet in frozen contract
rg -n "/negotiations/.*/accept|/negotiations/.*/reject|offer_history" docs/specs/openapi.yaml

# Compile backend crates touched by the spec
cd backend
cargo check -p server
cargo check -p api-contract
```
