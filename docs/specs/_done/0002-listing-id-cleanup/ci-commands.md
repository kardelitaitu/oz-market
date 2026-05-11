# CI Commands

Run from repo root:

```powershell
./check.ps1
```

Focused checks for this spec:

```powershell
# Ensure prefixed listing IDs are removed from tests
rg -n "product-\d+|service-\d+|property-\d+" backend/server/src

# Ensure listing_type assertions still exist in tests/support
rg -n "listing_type" backend/server/src

# Run test suite
cd backend
cargo test --lib
```
