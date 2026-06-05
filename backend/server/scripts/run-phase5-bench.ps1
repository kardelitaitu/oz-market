param(
    [string]$DatabaseUrl = $env:DATABASE_URL
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($DatabaseUrl)) {
    throw "DATABASE_URL is required to run the Postgres-backed phase5_bench profile."
}

$env:DATABASE_URL = $DatabaseUrl
$manifestPath = Resolve-Path (Join-Path $PSScriptRoot "..\..\Cargo.toml")

Write-Host "Running phase5_bench against Postgres-backed storage..."
Write-Host "Manifest: $($manifestPath.Path)"

& cargo run --manifest-path $manifestPath.Path -p oz-market-server --bin phase5_bench
exit $LASTEXITCODE
