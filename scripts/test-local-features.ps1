param(
    [string[]]$Crate = @(
        "codex-desktop-automation",
        "codex-first-moves",
        "codex-operation-cache",
        "codex-task-memory",
        "codex-self-review"
    ),
    [int]$Jobs = 1,
    [switch]$Timings,
    [int]$CleanupBelowGB = 5,
    [switch]$PreserveTestArtifacts
)

$ErrorActionPreference = "Stop"
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -Scope Global -ErrorAction SilentlyContinue) {
    $Global:PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$codexRs = Join-Path $repoRoot "codex-rs"
$logs = Join-Path $repoRoot "logs"
New-Item -ItemType Directory -Force -Path $logs | Out-Null

function Invoke-CleanSafeIfNeeded {
    param([switch]$IncludeTestArtifacts)

    $freeGB = [math]::Round((Get-PSDrive C).Free / 1GB, 2)
    if ($CleanupBelowGB -gt 0 -and $freeGB -ge $CleanupBelowGB) {
        return
    }

    $args = @(
        "-ExecutionPolicy", "Bypass",
        "-File", (Join-Path $repoRoot "scripts\build-local-codex.ps1"),
        "-Mode", "CleanSafe",
        "-CleanTestArtifactsBelowGB", "$CleanupBelowGB"
    )
    if ($IncludeTestArtifacts) {
        $args += "-CleanTestArtifacts"
    }

    & powershell @args | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "CleanSafe failed with exit code $LASTEXITCODE"
    }
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$summary = @()

Invoke-CleanSafeIfNeeded

foreach ($crateName in $Crate) {
    $safeName = $crateName -replace '[^A-Za-z0-9_.-]', '_'
    $log = Join-Path $logs "feature-test-$safeName-$stamp.log"
    $args = @("test", "--release", "-p", $crateName, "-j", "$Jobs")
    if ($Timings) {
        $args += "--timings"
    }

    Push-Location $codexRs
    try {
        $started = Get-Date
        Write-Host "Running cargo $($args -join ' ')"
        $cargoCommand = "cargo " + ($args -join " ")
        $cmdLine = "$cargoCommand > ""$log"" 2>&1"
        & cmd.exe /d /s /c $cmdLine
        $exitCode = $LASTEXITCODE
        Get-Content -Path $log
        $elapsed = (Get-Date) - $started
    } finally {
        Pop-Location
    }

    $summary += [pscustomobject]@{
        crate = $crateName
        exit_code = $exitCode
        seconds = [math]::Round($elapsed.TotalSeconds, 1)
        log = $log
    }

    if ($exitCode -ne 0) {
        $summary | ConvertTo-Json -Depth 3
        exit $exitCode
    }

    if (-not $PreserveTestArtifacts) {
        Invoke-CleanSafeIfNeeded -IncludeTestArtifacts
    }
}

$summary | ConvertTo-Json -Depth 3
