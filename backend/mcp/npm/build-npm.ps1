#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Build and package the Rust oz-market-mcp binary for npm distribution.
.DESCRIPTION
    Compiles the mcp server binary natively or cross-compiles using Docker.
    Saves output to backend/mcp/npm/binaries/[platform]/.
.PARAMETER Target
    The target compilation platform: native, win32, darwin, linux, or all.
.EXAMPLE
    .\build-npm.ps1 -Target native
#>

[CmdletBinding()]
param(
    [ValidateSet("win32", "darwin", "linux", "all", "native")]
    [string]$Target = "native",
    [switch]$Bump
)

$ErrorActionPreference = "Stop"
$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$backendDir = Resolve-Path "$scriptRoot\..\.."

# OS detection logic
$isWin = $IsWindows -or ($PSEdition -eq "Desktop") -or ($env:OS -match "Windows_NT")
$isMac = $IsMacOS
$isLin = $IsLinux

$hostPlatform = "win32"
$hostBinaryName = "oz-market-mcp.exe"
if ($isMac) {
    $hostPlatform = "darwin"
    $hostBinaryName = "oz-market-mcp"
} elseif ($isLin) {
    $hostPlatform = "linux"
    $hostBinaryName = "oz-market-mcp"
}

# Helper to check if Docker is running
function Test-DockerRunning {
    try {
        $null = docker ps -q 2>&1
        return ($LASTEXITCODE -eq 0)
    } catch {
        return $false
    }
}

# Helper to compile natively
function Build-Native {
    Write-Host "Building release binary natively for $hostPlatform..." -ForegroundColor Cyan
    Push-Location $backendDir
    try {
        cargo build --release --package oz-market-mcp
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo compilation failed with code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    $targetBinariesDir = Join-Path (Join-Path $scriptRoot "binaries") $hostPlatform
    if (-not (Test-Path $targetBinariesDir)) {
        $null = New-Item -ItemType Directory -Path $targetBinariesDir -Force
    }

    $sourcePath = Join-Path $backendDir "target\release\$hostBinaryName"
    $destPath = Join-Path $targetBinariesDir $hostBinaryName

    Write-Host "Copying native binary from $sourcePath to $destPath..."
    Copy-Item -Path $sourcePath -Destination $destPath -Force
}

# Helper to compile for Linux using Docker
function Build-LinuxDocker {
    Write-Host "Building release binary for Linux using Docker..." -ForegroundColor Cyan
    
    $dockerVolumePath = $backendDir.ToString()
    # Replace backslashes for Docker volume mapping if on Windows
    if ($isWin) {
        $dockerVolumePath = $dockerVolumePath -replace '\\', '/'
    }

    # Run docker build
    docker run --rm -v "${dockerVolumePath}:/app" -w /app rust:1.94-slim-bookworm sh -c "apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev curl ca-certificates && cargo build --release --package oz-market-mcp"
    if ($LASTEXITCODE -ne 0) {
        throw "Linux Docker compilation failed with code $LASTEXITCODE"
    }

    $targetBinariesDir = Join-Path $scriptRoot "binaries\linux"
    if (-not (Test-Path $targetBinariesDir)) {
        $null = New-Item -ItemType Directory -Path $targetBinariesDir -Force
    }

    $sourcePath = Join-Path $backendDir "target\release\oz-market-mcp"
    $destPath = Join-Path $targetBinariesDir "oz-market-mcp"

    Write-Host "Copying Linux binary from $sourcePath to $destPath..."
    Copy-Item -Path $sourcePath -Destination $destPath -Force
}

# Main Execution Flow
# 1. Version Bump Automation
$packageJsonPath = Join-Path $scriptRoot "package.json"
if (Test-Path $packageJsonPath) {
    $content = Get-Content $packageJsonPath -Raw
    if ($content -match '"version":\s*"(\d+)\.(\d+)\.(\d+)"') {
        $major = [int]$Matches[1]
        $minor = [int]$Matches[2]
        $patch = [int]$Matches[3]
        $currentVersion = "$major.$minor.$patch"
        
        Write-Host "Checking version status for $currentVersion..." -ForegroundColor Gray
        
        # Check npm registry to see if this version is already published
        $packageName = "@kardelitaitu/oz-market-mcp"
        $versionExists = $false
        try {
            $publishedJson = & npm view $packageName versions --json 2>$null
            if ($publishedJson) {
                $published = $publishedJson | ConvertFrom-Json
                [array]$publishedList = @()
                if ($published -is [array]) {
                    $publishedList = $published
                } elseif ($published -is [string]) {
                    $publishedList = @($published)
                }
                if ($publishedList -contains $currentVersion) {
                    $versionExists = $true
                }
            }
        } catch {
            # Package might not exist yet
        }
        
        if ($versionExists -or $Bump) {
            $newMajor = $major
            $newMinor = $minor
            $newPatch = $patch + 1
            
            if ($newPatch -gt 99) {
                $newPatch = 0
                $newMinor++
                if ($newMinor -gt 99) {
                    $newMinor = 0
                    $newMajor++
                }
            }
            $newVersion = "$newMajor.$newMinor.$newPatch"
            
            if ($versionExists) {
                Write-Host "Version $currentVersion is already published on the NPM registry. Auto-bumping version..." -ForegroundColor Yellow
                while ($publishedList -contains $newVersion) {
                    $newPatch++
                    if ($newPatch -gt 99) {
                        $newPatch = 0
                        $newMinor++
                        if ($newMinor -gt 99) {
                            $newMinor = 0
                            $newMajor++
                        }
                    }
                    $newVersion = "$newMajor.$newMinor.$newPatch"
                }
            } else {
                Write-Host "Manual version bump requested..." -ForegroundColor Cyan
            }
            
            $content = $content -replace '"version":\s*"\d+\.\d+\.\d+"', ('"version": "' + $newVersion + '"')
            $content | Out-File $packageJsonPath -Encoding utf8
            Write-Host "Successfully bumped package.json version to: $newVersion" -ForegroundColor Green
        } else {
            Write-Host "Version $currentVersion is available to publish. No auto-bump needed." -ForegroundColor Gray
        }
    }
}

Write-Host "Starting build execution with Target: $Target..." -ForegroundColor Green

switch ($Target) {
    "native" {
        Build-Native
    }
    "win32" {
        if (-not $isWin) {
            Write-Error "Cannot compile win32 binary natively on non-Windows host."
            exit 1
        }
        Build-Native
    }
    "darwin" {
        if (-not $isMac) {
            Write-Error "Cannot compile darwin binary natively on non-macOS host."
            exit 1
        }
        Build-Native
    }
    "linux" {
        if ($isLin) {
            Build-Native
        } else {
            if (Test-DockerRunning) {
                Build-LinuxDocker
            } else {
                Write-Error "To compile for Linux on a non-Linux host, Docker must be running."
                exit 1
            }
        }
    }
    "all" {
        Build-Native
        if ($isLin) {
            # Linux host compiles linux natively; nothing else to cross-compile natively
        } else {
            if (Test-DockerRunning) {
                Build-LinuxDocker
            } else {
                Write-Warning "Docker is not running. Skipping Linux build."
            }
        }
    }
}

Write-Host "NPM Package prepared successfully!" -ForegroundColor Green
Write-Host "To test locally, run: npx $scriptRoot"
