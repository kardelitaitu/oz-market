#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Auto-rust CI Checker - Runs full test suite like GitHub workflow
.DESCRIPTION
    Runs cargo check, fmt, clippy, then tests with detailed reporting.
    Short-circuits on first failure for fast feedback.
    Mirrors .github/workflows/ci.yml for local Windows development.
.EXAMPLE
    .\check.ps1           # Run all checks
    .\check.ps1 -SkipTests # Skip test execution
#>
[CmdletBinding()]
param(
    [switch]$SkipTests,
    [switch]$SkipClippy,
    [switch]$SkipFormat,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date

$colors = @{
    Green  = "`e[32m"
    Red    = "`e[31m"
    Yellow = "`e[33m"
    Blue   = "`e[34m"
    Cyan   = "`e[36m"
    Reset  = "`e[0m"
}

function Write-Status($msg, $color = "White") {
    $c = if ($colors[$color]) { $colors[$color] } else { "" }
    Write-Output "$c$msg$($colors.Reset)"
}

function Write-Header($title) {
    Write-Output ""
    Write-Status "=== $title ===" "Cyan"
}

$results = @{
    Build  = @{ Passed = $false; Duration = 0 }
    Format = @{ Passed = $false; Duration = 0 }
    Clippy = @{ Passed = $false; Duration = 0 }
    Tests  = @{ Passed = $false; Duration = 0 }
}

# Check if we're in the right directory (should have backend/Cargo.toml)
if (-not (Test-Path "backend/Cargo.toml")) {
    Write-Status "ERROR: Must run from project root (where backend/Cargo.toml exists)" "Red"
    exit 1
}

$failed = $false
$stepNum = 1

function Write-StepHeader($num, $desc) {
    Write-Output "$num. $desc"
}

function Write-StepResult($passed) {
    if ($passed) {
        Write-Status "PASS" "Green"
    } else {
        Write-Status "FAIL" "Red"
    }
}

# Change to backend directory (where Cargo.toml is)
$originalLocation = Get-Location
Set-Location "backend"

# ---- BUILD -----------------------------------------------------------
if (-not $SkipBuild) {
    $cmd = "cargo check"
    Write-StepHeader $stepNum "$cmd"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        cargo check 2>&1 | Out-Null
        $elapsed = $sw.Elapsed.TotalSeconds
        $passed = $LASTEXITCODE -eq 0
        $results.Build = @{ Passed = $passed; Duration = $elapsed }
        Write-StepResult $passed
        if (-not $passed) { $failed = $true }
    } catch {
        $elapsed = $sw.Elapsed.TotalSeconds
        $results.Build = @{ Passed = $false; Duration = $elapsed }
        Write-StepResult $false
        $failed = $true
    }
    $stepNum++
}

# ---- FORMAT -----------------------------------------------------------
if (-not $SkipFormat -and -not $failed) {
    $cmd = "cargo fmt --all -- --check"
    Write-StepHeader $stepNum "$cmd"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        cargo fmt --all -- --check 2>&1 | Out-Null
        $elapsed = $sw.Elapsed.TotalSeconds
        $passed = $LASTEXITCODE -eq 0
        $results.Format = @{ Passed = $passed; Duration = $elapsed }
        Write-StepResult $passed
        if (-not $passed) { 
            Write-Status "Run 'cargo fmt' to fix formatting" "Yellow"
            $failed = $true 
        }
    } catch {
        $elapsed = $sw.Elapsed.TotalSeconds
        $results.Format = @{ Passed = $false; Duration = $elapsed }
        Write-StepResult $false
        $failed = $true
    }
    $stepNum++
}

# ---- CLIPPY ----------------------------------------------------------
if (-not $SkipClippy -and -not $failed) {
    $cmd = "cargo clippy"
    Write-StepHeader $stepNum "$cmd"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        cargo clippy 2>&1 | Out-Null
        $elapsed = $sw.Elapsed.TotalSeconds
        $passed = $LASTEXITCODE -eq 0
        $results.Clippy = @{ Passed = $passed; Duration = $elapsed }
        Write-StepResult $passed
        if (-not $passed) { $failed = $true }
    } catch {
        $elapsed = $sw.Elapsed.TotalSeconds
        $results.Clippy = @{ Passed = $false; Duration = $elapsed }
        Write-StepResult $false
        $failed = $true
    }
    $stepNum++
}

# ---- TESTS ----------------------------------------------------------
if (-not $SkipTests -and -not $failed) {
    $cmd = "cargo test --lib"
    Write-StepHeader $stepNum "$cmd"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        cargo test --lib 2>&1 | Out-Null
        $elapsed = $sw.Elapsed.TotalSeconds
        $passed = $LASTEXITCODE -eq 0
        $results.Tests = @{ Passed = $passed; Duration = $elapsed }
        Write-StepResult $passed
        if (-not $passed) { $failed = $true }
    } catch {
        $elapsed = $sw.Elapsed.TotalSeconds
        $results.Tests = @{ Passed = $false; Duration = $elapsed }
        Write-StepResult $false
        $failed = $true
    }
    $stepNum++
}

# Return to original directory
Set-Location $originalLocation

# ---- REPORT ----------------------------------------------------------
$total = ((Get-Date) - $startTime).TotalSeconds
Write-Status "CI CHECKER REPORT:" "Yellow"
$p = 0; $f = 0
$runOrder = @("Build", "Format", "Clippy", "Tests")
foreach ($name in $runOrder) {
    $r = $results.$name
    if ($r.Duration -gt 0 -or $r.Passed) {
        $s = if ($r.Passed) { "PASS" } else { "FAIL" }
        $col = if ($r.Passed) { "Green" } else { "Red" }
        Write-Status ("{0,-8}  {1,-25}  {2,8}" -f $s, $name, "{0:N2}s" -f $r.Duration) $col
        if ($r.Passed) { $p++ } else { $f++ }
    }
}
Write-Status ("Passed: $p  |  Failed: $f  |  Total Time: {0:N2}s" -f $total) $(if ($f -eq 0) { "Green" } else { "Red" })
Write-Status "----------------------------------------------" "Cyan"

# ---- EXIT -----------------------------------------------------------
if ($f -eq 0) {
    Write-Status "All checks passed! Ready to commit (but don't push without asking!)" "Green"
    Write-Status "COMMIT REMINDER:" "Yellow"
    Write-Output "  - After making code changes, summarize the changes briefly"
    Write-Output "  - Append a short journal entry to JOURNAL.md"
    Write-Output "  - Journal entries should record what changed and why"
    Write-Output "  - NEVER git push without being specifically asked to do it"
    exit 0
} else {
    Write-Status "Some checks failed. Fix before committing." "Red"
    exit 1
}
