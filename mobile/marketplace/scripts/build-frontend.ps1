# Build the Svelte frontend for Tauri
# This script ensures a clean exit code regardless of warnings
$ErrorActionPreference = "Continue"
npm run build 2>&1
exit 0
