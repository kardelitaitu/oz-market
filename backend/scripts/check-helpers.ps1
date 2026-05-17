<#
.SYNOPSIS
    Pure helper functions for check.ps1 CI pipeline
.DESCRIPTION
    Extracted for unit testability with Pester.
    Dot-source this file in check.ps1 and in test files.
#>

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

    $diff = git -c core.safecrlf=false -C $RepoRoot diff --no-color HEAD -- $relativePath 2>$null
    if ($LASTEXITCODE -ne 0) {
        return @{ Passed = $false; Message = "Unable to diff $Path against HEAD" }
    }

    if (-not $diff) {
        return @{ Passed = $true; Message = "$Path is unchanged (append-only)" }
    }

    $hasRemovals = $diff | Select-String "^-" | Where-Object { $_ -notmatch "^\-\-\-" }
    if ($hasRemovals) {
        return @{ Passed = $false; Message = "$Path changed before the append-only boundary" }
    }

    return @{ Passed = $true; Message = "$Path is append-only" }
}

function Test-ActiveSpecGovernance {
    param(
        [string]$RepoRoot
    )

    $activeRoot = Join-Path $RepoRoot "docs/specs/_active"
    if (-not (Test-Path $activeRoot)) {
        return @{ Passed = $false; Message = "docs/specs/_active is missing" }
    }

    $issues = New-Object System.Collections.Generic.List[string]
    $files = Get-ChildItem -Path $activeRoot -Recurse -File -Include *.md,*.yaml,*.yml

    foreach ($file in $files) {
        $content = Get-Content -Path $file.FullName -Raw
        $relative = $file.FullName.Substring($RepoRoot.Length + 1).Replace('\', '/')
        $lines = $content -split "`r?`n"

        for ($i = 0; $i -lt $lines.Length; $i++) {
            if ($lines[$i] -match "docs/whitepaper/") {
                $issues.Add("${relative}:$($i + 1) uses legacy docs/whitepaper path")
            }
            if ($lines[$i] -match "Implementation details to be defined during active development\.") {
                $issues.Add("${relative}:$($i + 1) contains placeholder governance text")
            }
        }

        if ($file.Name -ieq "README.md") {
            $frontmatterStatus = [regex]::Match($content, "(?m)^status:\s*(.+)$").Groups[1].Value.Trim()
            $bodyStatus = [regex]::Match($content, '(?m)^Status:\s*`([^`]+)`').Groups[1].Value.Trim()
            if ($frontmatterStatus -and $bodyStatus -and $frontmatterStatus -ne $bodyStatus) {
                $issues.Add("$relative has status mismatch (frontmatter=$frontmatterStatus, body=$bodyStatus)")
            }

            $frontmatterImplementer = [regex]::Match($content, "(?m)^implementer:\s*(.+)$").Groups[1].Value.Trim()
            $bodyImplementer = [regex]::Match($content, '(?m)^Implementer:\s*`([^`]+)`').Groups[1].Value.Trim()
            if ($frontmatterImplementer -and $bodyImplementer -and $frontmatterImplementer -ne $bodyImplementer) {
                $issues.Add("$relative has implementer mismatch (frontmatter=$frontmatterImplementer, body=$bodyImplementer)")
            }
        }
    }

    if ($issues.Count -gt 0) {
        $preview = ($issues | Select-Object -First 10) -join "; "
        $suffix = if ($issues.Count -gt 10) { " (and $($issues.Count - 10) more)" } else { "" }
        return @{ Passed = $false; Message = "Active spec governance failed: $preview$suffix" }
    }

    return @{ Passed = $true; Message = "Active spec governance checks passed" }
}
