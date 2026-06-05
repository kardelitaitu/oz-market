param(
    [string]$DatabaseUrl = $env:DATABASE_URL,
    [switch]$SkipBootstrap
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($DatabaseUrl)) {
    $DatabaseUrl = "postgres://marketplace:marketplace@127.0.0.1:5432/marketplace?sslmode=disable"
}

$env:DATABASE_URL = $DatabaseUrl
$exitCode = 0

Push-Location (Join-Path $PSScriptRoot "..")
try {
    $manifestPath = Resolve-Path (Join-Path $PSScriptRoot "..\..\Cargo.toml")
    if (-not $SkipBootstrap) {
        & cargo run --manifest-path $manifestPath.Path -p oz-market-server --bin bootstrap_schema
        if ($LASTEXITCODE -ne 0) {
            throw "schema bootstrap failed"
        }
    }

    & "$PSScriptRoot\run-phase5-bench.ps1" -DatabaseUrl $DatabaseUrl
    $exitCode = $LASTEXITCODE
}
finally {
    Pop-Location
}

exit $exitCode
