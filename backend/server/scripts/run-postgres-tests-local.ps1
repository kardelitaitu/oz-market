param(
    [string]$DatabaseUrl = $env:DATABASE_URL
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($DatabaseUrl)) {
    $DatabaseUrl = "postgres://marketplace:marketplace@127.0.0.1:5432/marketplace?sslmode=disable"
}

$env:DATABASE_URL = $DatabaseUrl

Push-Location (Join-Path $PSScriptRoot "..")
try {
    $manifestPath = Resolve-Path (Join-Path $PSScriptRoot "..\..\Cargo.toml")
    & cargo test --manifest-path $manifestPath.Path -p marketplace-server --test postgres_flows
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
