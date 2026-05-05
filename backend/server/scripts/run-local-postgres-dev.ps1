param(
    [switch]$SkipBenchmark,
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"

try {
    & docker info | Out-Null
} catch {
    throw "Docker Desktop or another Docker daemon must be running to start local Postgres."
}

Write-Host "Starting local Postgres container..."
docker compose -p marketplace -f (Join-Path $PSScriptRoot "..\..\..\compose.postgres.yml") up -d postgres
if ($LASTEXITCODE -ne 0) {
    throw "failed to start local Postgres"
}

if (-not $SkipBenchmark) {
    Write-Host "Running phase5 benchmark against local Postgres..."
    & "$PSScriptRoot\run-phase5-bench-local.ps1"
    if ($LASTEXITCODE -ne 0) {
        throw "phase5 benchmark failed"
    }
}

if (-not $SkipTests) {
    Write-Host "Running Postgres integration tests..."
    & "$PSScriptRoot\run-postgres-tests-local.ps1"
    if ($LASTEXITCODE -ne 0) {
        throw "postgres integration tests failed"
    }
}

Write-Host "Local Postgres dev workflow completed."
