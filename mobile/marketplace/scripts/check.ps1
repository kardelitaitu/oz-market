<#
.SYNOPSIS
    Smoke test for the Tauri mobile app — verifies both Rust and Svelte build correctly.
.DESCRIPTION
    Runs cargo check, npm run build, cargo fmt --check, and cargo clippy.
    Short-circuits on first failure for fast feedback.
.EXAMPLE
    .\scripts\check.ps1
#>
$ErrorActionPreference = "Stop"
$startTime = Get-Date

# Calculate paths relative to this script
$scriptsDir = Split-Path -Parent $PSCommandPath
$mobileDir = Split-Path -Parent $scriptsDir
$tauriDir = Join-Path $mobileDir "src-tauri"

function Write-StepHeader($num, $desc) {
    Write-Output "$num. $desc"
}

$failed = $false
$stepNum = 1
$originalDir = Get-Location

# ---- CARGO CHECK -----------------------------------------------------------
Write-StepHeader $stepNum "cargo check"
try {
    Set-Location $tauriDir
    cargo check 2>&1 | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "cargo check failed" }
    Set-Location $originalDir
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
    Set-Location $tauriDir
    cargo fmt -- --check 2>&1 | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt --check failed" }
    Set-Location $originalDir
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
    Set-Location $tauriDir
    cargo clippy --all-targets -- -D warnings 2>&1 | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed" }
    Set-Location $originalDir
    Write-Output "PASS"
} catch {
    Set-Location $originalDir
    Write-Output "FAIL — fix warnings before committing"
    $failed = $true
}
$stepNum++
if ($failed) { exit 1 }

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
