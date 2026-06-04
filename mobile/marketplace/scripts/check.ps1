<#
.SYNOPSIS
    Smoke test for the Tauri mobile app — verifies both Rust and Svelte build correctly.
.DESCRIPTION
    Runs cargo check, npm run build, cargo fmt --check, and cargo clippy.
    Short-circuits on first failure for fast feedback.
.EXAMPLE
    .\scripts\check.ps1
#>
# Use "Continue" rather than "Stop" for cargo invocations: cargo writes
# informational output (Finished, Compiling) to stderr, which PowerShell
# would otherwise convert to a terminating NativeCommandError even when
# the command exits 0. Steps check $LASTEXITCODE explicitly.
$ErrorActionPreference = "Continue"
$startTime = Get-Date

# Calculate paths relative to this script
$scriptsDir = Split-Path -Parent $PSCommandPath
$mobileDir = Split-Path -Parent $scriptsDir
$tauriDir = Join-Path $mobileDir "src-tauri"

# On Windows hosts the system HTTP_PROXY env var intercepts 127.0.0.1
# traffic and returns 403 for mock servers. Nullify it for this run so
# wiremock/TcpListener-based integration tests can reach local listeners.
# Belt-and-braces: tests should also use per-client .no_proxy() to be
# robust in other CI environments.
$env:HTTP_PROXY = ""
$env:HTTPS_PROXY = ""
$env:http_proxy = ""
$env:https_proxy = ""

function Write-StepHeader($num, $desc) {
    Write-Output "$num. $desc"
}

function Invoke-CargoStep {
    param([string]$tauriDir, [string[]]$cargoArgs, [string]$failureMessage)
    Set-Location $tauriDir
    & cargo @cargoArgs 2>&1 | Out-Host
    if ($LASTEXITCODE -ne 0) { throw $failureMessage }
    Set-Location $originalDir
}

$failed = $false
$stepNum = 1
$originalDir = Get-Location

# ---- CARGO CHECK -----------------------------------------------------------
Write-StepHeader $stepNum "cargo check"
try {
    Invoke-CargoStep -tauriDir $tauriDir -cargoArgs @("check") -failureMessage "cargo check failed"
    Write-Output "PASS"
} catch {
    Set-Location $originalDir
    Write-Output "FAIL"
    Write-Output $_.Exception.Message
    $failed = $true
}
$stepNum++
if ($failed) { exit 1 }

# ---- CARGO FMT --------------------------------------------------------------
Write-StepHeader $stepNum "cargo fmt --check"
try {
    Invoke-CargoStep -tauriDir $tauriDir -cargoArgs @("fmt", "--", "--check") -failureMessage "cargo fmt --check failed"
    Write-Output "PASS"
} catch {
    Set-Location $originalDir
    Write-Output "FAIL — run 'cargo fmt' to fix"
    $failed = $true
}
$stepNum++
if ($failed) { exit 1 }

# ---- CARGO CLIPPY ----------------------------------------------------------
Write-StepHeader $stepNum "cargo clippy"
try {
    Invoke-CargoStep -tauriDir $tauriDir -cargoArgs @("clippy", "--all-targets", "--", "-D", "warnings") -failureMessage "cargo clippy failed"
    Write-Output "PASS"
} catch {
    Set-Location $originalDir
    Write-Output "FAIL — fix warnings before committing"
    $failed = $true
}
$stepNum++
if ($failed) { exit 1 }

# ---- CARGO TEST ------------------------------------------------------------
Write-StepHeader $stepNum "cargo test --lib"
try {
    Invoke-CargoStep -tauriDir $tauriDir -cargoArgs @("test", "--lib") -failureMessage "cargo test --lib failed"
    Write-Output "PASS"
} catch {
    Set-Location $originalDir
    Write-Output "FAIL"
    Write-Output $_.Exception.Message
    $failed = $true
}
$stepNum++

# ---- NPM BUILD --------------------------------------------------------------
Write-StepHeader $stepNum "npm run build"
try {
    Set-Location $mobileDir
    npm run build 2>&1 | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "npm run build failed" }
    Set-Location $originalDir
    Write-Output "PASS"
} catch {
    Set-Location $originalDir
    Write-Output "FAIL"
    $failed = $true
}
$stepNum++

# ---- REPORT ----------------------------------------------------------------
$total = ((Get-Date) - $startTime).TotalSeconds
if ($failed) {
    Write-Output "FAILED in $total s"
    exit 1
} else {
    Write-Output "ALL CHECKS PASSED in $total s"
    exit 0
}
