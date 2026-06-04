# CI Commands - Update Affected Documents

Execute these commands to verify documentation governance:

```bash
# Verify active spec governance checks pass
powershell -File check.ps1 -SkipBuild -SkipFormat -SkipClippy -SkipTests
```
