[CmdletBinding()]
param(
    [string]$RepoRoot,

    [Parameter(Mandatory = $true)]
    [string]$Package,

    [string]$Filter,

    [int]$Jobs = 1,

    [string[]]$ExtraCargoArgs = @(),

    [switch]$Lib,

    [switch]$AllowIntegrationTargets,

    [switch]$NoCleanup,

    [int]$CleanTestArtifactsBelowGB = 0
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$codexRs = Join-Path $RepoRoot "codex-rs"
$logs = Join-Path $RepoRoot "logs"
$buildScript = Join-Path $RepoRoot "scripts\build-local-codex.ps1"
New-Item -ItemType Directory -Force -Path $logs | Out-Null

$status = & powershell -NoProfile -ExecutionPolicy Bypass -File $buildScript -Mode Status | ConvertFrom-Json
if ($status.active_build_processes.Count -gt 0) {
    throw "A Cargo/rustc/link process is already active in this repo. Wait for it before starting a release test."
}

if ($Package -eq "codex-core" -and -not $Lib -and -not $AllowIntegrationTargets) {
    throw "Refusing to run codex-core package tests without -Lib. This would compile core/tests/all.rs before applying any filter. Use -Lib for unit tests or -AllowIntegrationTargets when intentionally testing integration targets."
}

if (-not $NoCleanup) {
    Write-Host "Pre-test cleanup: pruning orphaned release dependency artifacts."
    & powershell -NoProfile -ExecutionPolicy Bypass -File $buildScript -Mode PruneReleaseDeps
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$safePackage = $Package -replace '[^A-Za-z0-9_.-]', '-'
$safeFilter = if ([string]::IsNullOrWhiteSpace($Filter)) { "all" } else { $Filter -replace '[^A-Za-z0-9_.-]', '-' }
$log = Join-Path $logs "test-local-release-$safePackage-$safeFilter-$timestamp.log"

$cargoArgs = @("test", "-p", $Package, "--release")
if ($Jobs -gt 0) {
    $cargoArgs += @("-j", "$Jobs")
}
if ($Lib) {
    $cargoArgs += "--lib"
}
if (-not [string]::IsNullOrWhiteSpace($Filter)) {
    $cargoArgs += $Filter
}
$cargoArgs += $ExtraCargoArgs

Write-Host ("Running: cargo {0}" -f ($cargoArgs -join " "))
Write-Host "Log: $log"

$testExit = 1
$hadNativeCommandPreference = Test-Path -LiteralPath Variable:\PSNativeCommandUseErrorActionPreference
if ($hadNativeCommandPreference) {
    $previousNativeCommandPreference = $PSNativeCommandUseErrorActionPreference
    $PSNativeCommandUseErrorActionPreference = $false
}
Push-Location $codexRs
try {
    & cargo @cargoArgs 2>&1 | Tee-Object -FilePath $log
    $testExit = $LASTEXITCODE
}
finally {
    Pop-Location
    if ($hadNativeCommandPreference) {
        $PSNativeCommandUseErrorActionPreference = $previousNativeCommandPreference
    }
}
if ($null -eq $testExit) {
    $testExit = 1
}

if ($testExit -eq 0 -and -not $NoCleanup) {
    Write-Host "Release test passed; cleaning disposable release test executable artifacts."
    & powershell -NoProfile -ExecutionPolicy Bypass -File $buildScript -Mode CleanSafe -CleanTestArtifacts -CleanTestArtifactsBelowGB $CleanTestArtifactsBelowGB
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
elseif ($testExit -ne 0) {
    Write-Host "Release test failed; keeping test artifacts for diagnosis."
    if (-not $NoCleanup) {
        Write-Host "Post-test cleanup: pruning orphaned release dependency artifacts while preserving test artifacts."
        & powershell -NoProfile -ExecutionPolicy Bypass -File $buildScript -Mode PruneReleaseDeps
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
}

exit $testExit
