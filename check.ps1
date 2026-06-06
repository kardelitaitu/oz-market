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

function Write-Status($msg, $color = "White") {
    Write-Output $msg
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
    Journal = @{ Passed = $false; Duration = 0 }
    ActiveSpecs = @{ Passed = $false; Duration = 0 }
    BenchGate = @{ Passed = $false; Duration = 0 }
}

# Dot-source pure helper functions for testability
. "$PSScriptRoot\backend\scripts\check-helpers.ps1"

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
$repoRoot = $originalLocation.Path
Set-Location "backend"

# ---- JOURNAL APPEND-ONLY GUARD --------------------------------------
$cmd = "journal append-only guard"
Write-StepHeader $stepNum $cmd
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $journalPath = Join-Path $repoRoot "JOURNAL.md"
    $journalCheck = Test-JournalAppendOnly -RepoRoot $repoRoot -Path $journalPath
    $elapsed = $sw.Elapsed.TotalSeconds
    $results.Journal = @{ Passed = $journalCheck.Passed; Duration = $elapsed }
    Write-StepResult $journalCheck.Passed
    if (-not $journalCheck.Passed) {
        Write-Status $journalCheck.Message "Yellow"
        $failed = $true
    }
} catch {
    $elapsed = $sw.Elapsed.TotalSeconds
    $results.Journal = @{ Passed = $false; Duration = $elapsed }
    Write-StepResult $false
    Write-Status "Journal guard error: $($_.Exception.Message)" "Yellow"
    $failed = $true
}
$stepNum++

# ---- ACTIVE SPEC GOVERNANCE GUARD ------------------------------------
$cmd = "active spec governance guard"
Write-StepHeader $stepNum $cmd
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $activeSpecCheck = Test-ActiveSpecGovernance -RepoRoot $repoRoot
    $elapsed = $sw.Elapsed.TotalSeconds
    $results.ActiveSpecs = @{ Passed = $activeSpecCheck.Passed; Duration = $elapsed }
    Write-StepResult $activeSpecCheck.Passed
    if (-not $activeSpecCheck.Passed) {
        Write-Status $activeSpecCheck.Message "Yellow"
        $failed = $true
    }
} catch {
    $elapsed = $sw.Elapsed.TotalSeconds
    $results.ActiveSpecs = @{ Passed = $false; Duration = $elapsed }
    Write-StepResult $false
    $failed = $true
}
$stepNum++

# ---- BUILD -----------------------------------------------------------
if (-not $SkipBuild) {
    $cmd = "cargo check"
    Write-StepHeader $stepNum "$cmd"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        cargo check
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
        cargo fmt --all -- --check
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
    $cmd = "cargo clippy --workspace --all-targets -- -D warnings"
    Write-StepHeader $stepNum "$cmd"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        cargo clippy --workspace --all-targets -- -D warnings
        $elapsed = $sw.Elapsed.TotalSeconds
        $passed = $LASTEXITCODE -eq 0
        $results.Clippy = @{ Passed = $passed; Duration = $elapsed }
        Write-StepResult $passed
        if (-not $passed) { 
            Write-Status "Run 'cargo clippy --workspace --all-targets -- -D warnings' to fix warnings" "Yellow"
            $failed = $true 
        }
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
        cargo test --lib
        $passed = $LASTEXITCODE -eq 0
        $elapsed = $sw.Elapsed.TotalSeconds
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

# ---- FRONTEND BUILD (Playwright E2E) ----------------------------------
if (-not $SkipTests -and -not $failed) {
    $websiteDir = Join-Path $repoRoot "web/website"
    if (Test-Path (Join-Path $websiteDir "package.json")) {
        $cmd = "npm run build (oz-market-website)"
        Write-StepHeader $stepNum "$cmd"
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $buildOk = $true
        try {
            Push-Location $websiteDir
            $output = npm run build 2>&1
            $buildOk = $LASTEXITCODE -eq 0
            if (-not $buildOk) { Write-Host $output }
            if ($buildOk) {
                npx playwright install chromium --with-deps 2>$null
                $env:NO_E2E_WEBSERVER = "1"
                # Check if a preview server is already running
                # NOTE: Must use Node.js http module — Vite 8 blocks non-browser HTTP
                $serverAlreadyRunning = $false
                node -e "const h=require('http');h.get('http://127.0.0.1:4173/',()=>process.exit(0)).on('error',()=>process.exit(1))" 2>$null
                if ($LASTEXITCODE -eq 0) { $serverAlreadyRunning = $true }
                if (-not $serverAlreadyRunning) {
                    $serverJob = Start-Job -ScriptBlock { param($dir) Set-Location $dir; npx vite preview --port 4173 --host 127.0.0.1 } -ArgumentList $websiteDir
                    $ready = $false
                    for ($i = 0; $i -lt 60; $i++) {
                        Start-Sleep -Milliseconds 500
                        node -e "const h=require('http');h.get('http://127.0.0.1:4173/',()=>process.exit(0)).on('error',()=>process.exit(1))" 2>$null
                        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
                    }
                    if (-not $ready) { throw "vite preview did not start within 30s" }
                }
                $output = npx playwright test 2>&1
                $buildOk = $LASTEXITCODE -eq 0
                if (-not $buildOk) { Write-Host $output }
                if (-not $serverAlreadyRunning) {
                    Stop-Job $serverJob -ErrorAction SilentlyContinue
                    Remove-Job $serverJob -ErrorAction SilentlyContinue
                }
                Remove-Item Env:\NO_E2E_WEBSERVER -ErrorAction SilentlyContinue
            }
            Pop-Location
            $elapsed = $sw.Elapsed.TotalSeconds
            $results.Website = @{ Passed = $buildOk; Duration = $elapsed }
            Write-StepResult $buildOk
            if (-not $buildOk) { $failed = $true }
        } catch {
            if (-not $serverAlreadyRunning) {
                Stop-Job $serverJob -ErrorAction SilentlyContinue
                Remove-Job $serverJob -ErrorAction SilentlyContinue
            }
            Remove-Item Env:\NO_E2E_WEBSERVER -ErrorAction SilentlyContinue
            Pop-Location
            $elapsed = $sw.Elapsed.TotalSeconds
            $results.Website = @{ Passed = $false; Duration = $elapsed }
            Write-StepResult $false
            $failed = $true
        }
        $stepNum++
    }
}

# ---- BENCHMARK CI GATE ------------------------------------------------
if (-not $failed) {
    $cmd = "benchmark ci gate (bench_suite --check)"
    Write-StepHeader $stepNum "$cmd"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $output = cargo run --bin bench_suite -- --check 2>&1
        $passed = $LASTEXITCODE -eq 0
        if (-not $passed) { Write-Host $output }
        $elapsed = $sw.Elapsed.TotalSeconds
        $results.BenchGate = @{ Passed = $passed; Duration = $elapsed }
        Write-StepResult $passed
        if (-not $passed) { $failed = $true }
    } catch {
        $elapsed = $sw.Elapsed.TotalSeconds
        $results.BenchGate = @{ Passed = $false; Duration = $elapsed }
        Write-StepResult $false
        $failed = $true
    }
    Set-Location $originalLocation
    $stepNum++
}

# Return to original directory
Set-Location $originalLocation

# ---- REPORT ----------------------------------------------------------
$total = ((Get-Date) - $startTime).TotalSeconds
Write-Status "CI CHECKER REPORT:" "Yellow"
$p = 0; $f = 0
$runOrder = @("Journal", "ActiveSpecs", "Build", "Format", "Clippy", "Tests", "Website", "BenchGate")
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
