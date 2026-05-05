param(
    [string]$DatabaseUrl = $env:DATABASE_URL
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($DatabaseUrl)) {
    $DatabaseUrl = "postgres://marketplace:marketplace@127.0.0.1:5432/marketplace?sslmode=disable"
}

& "$PSScriptRoot\run-phase5-bench.ps1" -DatabaseUrl $DatabaseUrl
exit $LASTEXITCODE
