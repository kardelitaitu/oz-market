# CI Commands

Run from repo root:

```powershell
./check.ps1
```

Focused checks for this spec:

```powershell
# Validate listing route contract surface
rg -n "/listings/\{listing_id\}|operationId: getListing|ListingSummary" docs/specs/openapi.yaml

# Validate no legacy type-specific listing paths in frozen contract
rg -n "/product/|/service/|/property/" docs/specs/openapi.yaml

# Validate handler and contract modules still compile
cd backend
cargo check -p server
cargo check -p api-contract
```
