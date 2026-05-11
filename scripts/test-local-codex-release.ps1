[CmdletBinding()]
param(
    [string]$RepoRoot,

    [Parameter(Mandatory = $true)]
    [string]$Package,

    [string]$Filter,

    [int]$Jobs = 1,

    [string[]]$ExtraCargoArgs = @(),

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

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$safePackage = $Package -replace '[^A-Za-z0-9_.-]', '-'
$safeFilter = if ([string]::IsNullOrWhiteSpace($Filter)) { "all" } else { $Filter -replace '[^A-Za-z0-9_.-]', '-' }
$log = Join-Path $logs "test-local-release-$safePackage-$safeFilter-$timestamp.log"

$cargoArgs = @("test", "-p", $Package, "--release")
if ($Jobs -gt 0) {
    $cargoArgs += @("-j", "$Jobs")
}
if (-not [string]::IsNullOrWhiteSpace($Filter)) {
    $cargoArgs += $Filter
}
$cargoArgs += $ExtraCargoArgs

Write-Host ("Running: cargo {0}" -f ($cargoArgs -join " "))
Write-Host "Log: $log"

$testExit = 1
Push-Location $codexRs
try {
    & cargo @cargoArgs 2>&1 | Tee-Object -FilePath $log
    $testExit = $LASTEXITCODE
}
finally {
    Pop-Location
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
}

exit $testExit
