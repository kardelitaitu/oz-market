#!/usr/bin/env pwsh

$ErrorActionPreference = "Stop"
$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

# 1. Compile the Rust binary in release mode
Write-Host "Building release binary..."
cd "$scriptRoot\..\.."
cargo build --release --package oz-market-mcp

# 2. Setup folder structure
Write-Host "Setting up folder structure..."
$targetBinariesDir = "$scriptRoot\binaries\win32"
if (-not (Test-Path $targetBinariesDir)) {
    New-Item -ItemType Directory -Path $targetBinariesDir
}

# 3. Copy binary
Write-Host "Copying binary..."
Copy-Item -Path "target\release\oz-market-mcp.exe" -Destination "$targetBinariesDir\oz-market-mcp.exe" -Force

Write-Host "NPM Package prepared successfully!"
Write-Host "To test locally, run: npx $scriptRoot"
