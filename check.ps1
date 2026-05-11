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

function Test-JournalAppendOnly {
    param(
        [string]$RepoRoot,
        [string]$Path = "JOURNAL.md"
    )

    $fullPath = if ([System.IO.Path]::IsPathRooted($Path)) { $Path } else { Join-Path $RepoRoot $Path }
    $relativePath = "JOURNAL.md"

    if (-not (Test-Path $fullPath)) {
        return @{ Passed = $false; Message = "$Path is missing" }
    }

    # Use git diff HEAD to detect modifications to existing content.
    # In unified-diff output, removed lines start with '-'; added lines
    # start with '+'.  An append-only journal will have only '+' lines
    # (the '--- a/...' header is excluded).  If any '-' lines appear,
    # previously committed content was modified.
    $diff = git -C $RepoRoot diff HEAD -- $relativePath 2>$null
    if ($LASTEXITCODE -ne 0) {
        return @{ Passed = $false; Message = "Unable to diff $Path against HEAD" }
    }

    if (-not $diff) {
        return @{ Passed = $true; Message = "$Path is unchanged (append-only)" }
    }

    # Look for any line that starts with '-' but not '---' (the latter is
    # the diff header).  This is encoding-safe because git handles the
    # byte-level comparison.
    $hasRemovals = $diff | Select-String "^\-" | Where-Object { $_ -notmatch "^\-\-\-" }
    if ($hasRemovals) {
        return @{ Passed = $false; Message = "$Path changed before the append-only boundary" }
    }

    return @{ Passed = $true; Message = "$Path is append-only" }
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
$runOrder = @("Journal", "Build", "Format", "Clippy", "Tests")
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
