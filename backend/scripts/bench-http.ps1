param(
    [string]$DatabaseUrl = $env:DATABASE_URL,
    [string]$BaseUrl = "http://127.0.0.1:3000",
    [int]$Ops = 1000,
    [string]$ConcurrencyLevels = "100,200,500",
    [switch]$SeedDatabase
)

$ErrorActionPreference = "Stop"

$ServerJob = $null
$BenchmarkResult = $null

function Cleanup {
    if ($ServerJob) {
        Write-Host "Stopping server..." -ForegroundColor Yellow
        Stop-Job $ServerJob -ErrorAction SilentlyContinue
        Remove-Job $ServerJob -ErrorAction SilentlyContinue
    }
}

trap { Cleanup; exit 1 }

if ([string]::IsNullOrWhiteSpace($DatabaseUrl)) {
    $DatabaseUrl = "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"
}

try {
    docker info | Out-Null
} catch {
    Write-Host "ERROR: Docker is not running. Please start Docker first." -ForegroundColor Red
    exit 1
}

Write-Host "Starting Postgres via docker compose..." -ForegroundColor Cyan
Set-Location "C:/My Script/project-the-marketplace/backend"
docker compose -p marketplace -f ../compose.postgres.yml up -d postgres
if ($LASTEXITCODE -ne 0) {
    throw "failed to start Postgres"
}

$env:DATABASE_URL = $DatabaseUrl
$env:MARKETPLACE_BIND = "127.0.0.1:3000"
$env:RUST_LOG = if ([string]::IsNullOrWhiteSpace($env:RUST_LOG)) { "info" } else { $env:RUST_LOG }

if ($SeedDatabase) {
    Write-Host "Seeding database with current generator..." -ForegroundColor Cyan
    cargo run --release --manifest-path Cargo.toml -p oz-market-server --bin populate_db
    if ($LASTEXITCODE -ne 0) {
        throw "database seeding failed"
    }
}

Write-Host "Starting Actix server (release)..." -ForegroundColor Cyan
$ServerJob = Start-Job -ScriptBlock {
    Set-Location "C:/My Script/project-the-marketplace/backend"
    $env:DATABASE_URL = "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"
    $env:MARKETPLACE_BIND = "127.0.0.1:3000"
    cargo run --release --bin oz-market-server
}

Write-Host "Waiting for server to be ready..." -ForegroundColor Yellow
$maxWait = 120
$waited = 0
$ready = $false

while ($waited -lt $maxWait) {
    try {
        $response = Invoke-WebRequest -Uri "$BaseUrl/health" -TimeoutSec 2 -ErrorAction SilentlyContinue
        if ($response.StatusCode -eq 200) {
            Write-Host "Server is ready!" -ForegroundColor Green
            $ready = $true
            break
        }
    } catch {}

    Start-Sleep -Seconds 1
    $waited++
    Write-Host "." -NoNewline
}

if (-not $ready) {
    Write-Host "`nERROR: Server did not become ready in time." -ForegroundColor Red
    Cleanup
    exit 1
}

Write-Host "`nRunning real HTTP benchmark..." -ForegroundColor Cyan
$env:HTTP_BENCH_OPS = "$Ops"
$env:HTTP_BENCH_CONCURRENCIES = $ConcurrencyLevels

cargo run --release --manifest-path Cargo.toml -p oz-market-server --bin bench_concurrent -- "$BaseUrl" "$Ops" "$ConcurrencyLevels"
$BenchmarkResult = $LASTEXITCODE

Cleanup

if ($BenchmarkResult -eq 0) {
    Write-Host "`n✅ Benchmark completed!" -ForegroundColor Green
} else {
    Write-Host "`n❌ Benchmark failed." -ForegroundColor Red
}

exit $BenchmarkResult
