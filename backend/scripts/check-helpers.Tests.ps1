#requires -Modules @{ ModuleName = 'Pester'; ModuleVersion = '5.0.0' }

Describe "Test-JournalAppendOnly" {
    BeforeAll {
        $helperPath = Join-Path (Split-Path -Parent $PSCommandPath) "check-helpers.ps1"
        . $helperPath

        $script:journalRepo = Join-Path $env:TEMP "pester-journal-$([System.IO.Path]::GetRandomFileName())"
        New-Item -Path $script:journalRepo -ItemType Directory -Force | Out-Null
        Push-Location $script:journalRepo
        git init 2>&1 | Out-Null
        git config user.email "test@test.com"
        git config user.name "Test"
        "initial" | Out-File "JOURNAL.md" -Encoding utf8
        git add .
        git commit -m "init" 2>&1 | Out-Null
        Pop-Location
    }

    AfterAll {
        if (Test-Path $script:journalRepo) { Remove-Item -Path $script:journalRepo -Recurse -Force }
    }

    It "returns fail when file is missing" {
        $result = Test-JournalAppendOnly -RepoRoot $script:journalRepo -Path "NONEXISTENT.md"
        $result.Passed | Should -Be $false
        $result.Message | Should -Match "missing"
    }

    It "returns pass when file is unchanged" {
        $result = Test-JournalAppendOnly -RepoRoot $script:journalRepo
        $result.Passed | Should -Be $true
        $result.Message | Should -Match "unchanged"
    }

    It "returns pass when only additions exist" {
        "new line" | Out-File "JOURNAL.md" -Encoding utf8 -Append
        $result = Test-JournalAppendOnly -RepoRoot $script:journalRepo
        $result.Passed | Should -Be $true
        $result.Message | Should -Match "append-only"
    }

    It "returns fail when removals exist" {
        git -C $script:journalRepo add . 2>&1 | Out-Null
        git -C $script:journalRepo commit -m "add line" 2>&1 | Out-Null
        "replacement" | Out-File (Join-Path $script:journalRepo "JOURNAL.md") -Encoding utf8
        $result = Test-JournalAppendOnly -RepoRoot $script:journalRepo
        $result.Passed | Should -Be $false
        $result.Message | Should -Match "append-only boundary"
    }

    It "returns fail when file is missing with explicit path" {
        $result = Test-JournalAppendOnly -RepoRoot $script:journalRepo -Path "NONEXISTENT.md"
        $result.Passed | Should -Be $false
        $result.Message | Should -Match "is missing"
    }
}

Describe "Test-ActiveSpecGovernance" {
    BeforeAll {
        $helperPath = Join-Path (Split-Path -Parent $PSCommandPath) "check-helpers.ps1"
        . $helperPath

        $script:specRoot = Join-Path $env:TEMP "pester-spec-$([System.IO.Path]::GetRandomFileName())"
    }

    AfterEach {
        $ad = Join-Path $script:specRoot "docs/specs/_active"
        if (Test-Path $ad) { Remove-Item -Path $ad -Recurse -Force }
    }

    AfterAll {
        if (Test-Path $script:specRoot) { Remove-Item -Path $script:specRoot -Recurse -Force }
    }

    It "returns fail when _active directory is missing" {
        $result = Test-ActiveSpecGovernance -RepoRoot $script:specRoot
        $result.Passed | Should -Be $false
        $result.Message | Should -Match "missing"
    }

    It "returns pass for a clean active spec directory" {
        $ad = Join-Path $script:specRoot "docs/specs/_active"
        New-Item -Path $ad -ItemType Directory -Force | Out-Null
        "status: draft`n`nThis is a clean spec." | Out-File "$ad\clean.md" -Encoding utf8
        $result = Test-ActiveSpecGovernance -RepoRoot $script:specRoot
        $result.Passed | Should -Be $true
    }

    It "fails when active spec references legacy docs/whitepaper path" {
        $ad = Join-Path $script:specRoot "docs/specs/_active"
        New-Item -Path $ad -ItemType Directory -Force | Out-Null
        "See docs/whitepaper/README.md" | Out-File "$ad\bad.md" -Encoding utf8
        $result = Test-ActiveSpecGovernance -RepoRoot $script:specRoot
        $result.Passed | Should -Be $false
        $result.Message | Should -Match "whitepaper"
    }

    It "fails when active spec contains placeholder governance text" {
        $ad = Join-Path $script:specRoot "docs/specs/_active"
        New-Item -Path $ad -ItemType Directory -Force | Out-Null
        "Implementation details to be defined during active development." | Out-File "$ad\placeholder.md" -Encoding utf8
        $result = Test-ActiveSpecGovernance -RepoRoot $script:specRoot
        $result.Passed | Should -Be $false
        $result.Message | Should -Match "placeholder"
    }

    It "fails when README has status mismatch" {
        $ad = Join-Path $script:specRoot "docs/specs/_active"
        New-Item -Path $ad -ItemType Directory -Force | Out-Null
        $c = "---`nstatus: draft`n---`n# Spec`nStatus: ``active``"
        $c | Out-File "$ad\README.md" -Encoding utf8
        $result = Test-ActiveSpecGovernance -RepoRoot $script:specRoot
        $result.Passed | Should -Be $false
        $result.Message | Should -Match "status mismatch"
    }

    It "fails when README has implementer mismatch" {
        $ad = Join-Path $script:specRoot "docs/specs/_active"
        New-Item -Path $ad -ItemType Directory -Force | Out-Null
        $c = "---`nimplementer: alice`n---`n# Spec`nImplementer: ``bob``"
        $c | Out-File "$ad\README.md" -Encoding utf8
        $result = Test-ActiveSpecGovernance -RepoRoot $script:specRoot
        $result.Passed | Should -Be $false
        $result.Message | Should -Match "implementer mismatch"
    }

    It "scans yaml files too" {
        $ad = Join-Path $script:specRoot "docs/specs/_active"
        New-Item -Path $ad -ItemType Directory -Force | Out-Null
        "docs/whitepaper/config" | Out-File "$ad\spec.yaml" -Encoding utf8
        $result = Test-ActiveSpecGovernance -RepoRoot $script:specRoot
        $result.Passed | Should -Be $false
    }

    It "reports multiple issues across multiple files" {
        $ad = Join-Path $script:specRoot "docs/specs/_active"
        New-Item -Path $ad -ItemType Directory -Force | Out-Null
        "docs/whitepaper/foo" | Out-File "$ad\a.md" -Encoding utf8
        "Implementation details to be defined during active development." | Out-File "$ad\b.md" -Encoding utf8
        $result = Test-ActiveSpecGovernance -RepoRoot $script:specRoot
        $result.Passed | Should -Be $false
        $result.Message | Should -Match "a\.md"
        $result.Message | Should -Match "b\.md"
    }
}
