# bench-http.ps1 - Run HTTP benchmark against Actix server
# Requires: Docker running with Postgres (docker-compose up -d)

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

# Check if Docker is running
try {
    docker info | Out-Null
} catch {
    Write-Host "ERROR: Docker is not running. Please start Docker first." -ForegroundColor Red
    exit 1
}

# Start Postgres if not running
Write-Host "Starting Postgres via docker-compose..." -ForegroundColor Cyan
Set-Location "C:/My Script/project-the-marketplace/backend"
docker-compose -p marketplace-local up -d postgres
Start-Sleep -Seconds 3

# Set environment variables
$env:DATABASE_URL = "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"
$env:MARKETPLACE_BIND = "127.0.0.1:3000"

# Start the server in background job
Write-Host "Starting Actix server (Phase 1 - Actix + Moka)..." -ForegroundColor Cyan
$ServerJob = Start-Job -ScriptBlock {
    Set-Location "C:/My Script/project-the-marketplace/backend"
    $env:DATABASE_URL = "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"
    $env:MARKETPLACE_BIND = "127.0.0.1:3000"
    cargo run --package marketplace-server
}

# Wait for server to be ready
Write-Host "Waiting for server to be ready..." -ForegroundColor Yellow
$maxWait = 30
$waited = 0
$ready = $false

while ($waited -lt $maxWait) {
    try {
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:3000/health" -TimeoutSec 2 -ErrorAction SilentlyContinue
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
    Write-Host "\nERROR: Server did not become ready in time." -ForegroundColor Red
    Cleanup
    exit 1
}

# Run the HTTP benchmark
Write-Host "\nRunning HTTP benchmark..." -ForegroundColor Cyan
$env:DATABASE_URL = "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable"

cargo run --package marketplace-server --bin http_bench -- "http://127.0.0.1:3000" 500

$BenchmarkResult = $LASTEXITCODE

# Cleanup
Cleanup

if ($BenchmarkResult -eq 0) {
    Write-Host "\n✅ Benchmark completed!" -ForegroundColor Green
} else {
    Write-Host "\n❌ Benchmark failed." -ForegroundColor Red
}

exit $BenchmarkResult
