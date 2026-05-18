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

    [switch]$AllowBroadTuiUnitTests,

    [switch]$AllowBroadCoreLibUnitTests,

    [switch]$CleanCoreLibTestArtifactsOnSuccess,

    [switch]$NoCleanup,

    [int]$CleanTestArtifactsBelowGB = 0
)

$ErrorActionPreference = "Stop"

function ConvertTo-NativeArgument {
    param([Parameter(Mandatory = $true)][string]$Value)

    if ($Value -notmatch '[\s"]') {
        return $Value
    }

    $escaped = $Value -replace '(\\*)"', '$1$1\"'
    $escaped = $escaped -replace '(\\+)$', '$1$1'
    return '"' + $escaped + '"'
}

function Read-NewText {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][ref]$Offset
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return ""
    }

    $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
    try {
        if ($stream.Length -le $Offset.Value) {
            return ""
        }

        [void]$stream.Seek($Offset.Value, [System.IO.SeekOrigin]::Begin)
        $reader = New-Object System.IO.StreamReader($stream, [System.Text.Encoding]::UTF8, $true)
        try {
            $text = $reader.ReadToEnd()
            $Offset.Value = $stream.Position
            return $text
        }
        finally {
            $reader.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Write-LoggedText {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string]$Text
    )

    if ([string]::IsNullOrEmpty($Text)) {
        return
    }

    Write-Host -NoNewline $Text
    [System.IO.File]::AppendAllText($Path, $Text, [System.Text.Encoding]::UTF8)
}

function Write-Heartbeat {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][System.Diagnostics.Stopwatch]$Stopwatch
    )

    $activeProcesses = Get-Process cargo,rustc,link -ErrorAction SilentlyContinue |
        Sort-Object ProcessName, Id |
        ForEach-Object { "{0}:{1}" -f $_.ProcessName, $_.Id }
    $processText = if ($activeProcesses) { $activeProcesses -join ", " } else { "none" }
    $message = "[test-local-codex-release] cargo still running after {0:n0}s; active build processes: {1}{2}" -f $Stopwatch.Elapsed.TotalSeconds, $processText, [Environment]::NewLine
    Write-LoggedText -Path $Path -Text $message
}

function Invoke-CargoWithHeartbeat {
    param(
        [Parameter(Mandatory = $true)][string[]]$CargoArgs,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [Parameter(Mandatory = $true)][string]$LogPath,
        [Parameter(Mandatory = $true)][string]$LogsDirectory
    )

    $tempBase = Join-Path $LogsDirectory ".test-local-codex-release"
    New-Item -ItemType Directory -Force -Path $tempBase | Out-Null
    $runId = [guid]::NewGuid().ToString("N")
    $stdout = Join-Path $tempBase "$runId.out"
    $stderr = Join-Path $tempBase "$runId.err"
    $stdoutOffset = 0L
    $stderrOffset = 0L
    [System.IO.File]::WriteAllText($LogPath, "", [System.Text.Encoding]::UTF8)
    $argumentList = ($CargoArgs | ForEach-Object { ConvertTo-NativeArgument $_ }) -join " "
    $process = Start-Process -FilePath "cargo" `
        -ArgumentList $argumentList `
        -WorkingDirectory $WorkingDirectory `
        -NoNewWindow `
        -PassThru `
        -RedirectStandardOutput $stdout `
        -RedirectStandardError $stderr
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $lastHeartbeat = Get-Date

    try {
        while (-not $process.HasExited) {
            Start-Sleep -Seconds 5
            Write-LoggedText -Path $LogPath -Text (Read-NewText -Path $stdout -Offset ([ref]$stdoutOffset))
            Write-LoggedText -Path $LogPath -Text (Read-NewText -Path $stderr -Offset ([ref]$stderrOffset))

            if (((Get-Date) - $lastHeartbeat).TotalSeconds -ge 30) {
                Write-Heartbeat -Path $LogPath -Stopwatch $stopwatch
                $lastHeartbeat = Get-Date
            }
        }

        $process.WaitForExit()
        Write-LoggedText -Path $LogPath -Text (Read-NewText -Path $stdout -Offset ([ref]$stdoutOffset))
        Write-LoggedText -Path $LogPath -Text (Read-NewText -Path $stderr -Offset ([ref]$stderrOffset))
        return $process.ExitCode
    }
    finally {
        if ($null -ne $process -and -not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            $process.WaitForExit()
        }
        Remove-Item -LiteralPath $stdout, $stderr -Force -ErrorAction SilentlyContinue
    }
}

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
if ($Package -eq "codex-core" -and $Lib -and -not [string]::IsNullOrWhiteSpace($Filter) -and -not $AllowBroadCoreLibUnitTests) {
    throw "Refusing filtered codex-core --lib test without -AllowBroadCoreLibUnitTests. Cargo still compiles and links the full codex-core unit-test harness before applying the filter, so a single filtered test can look stuck or time out. Prefer the smaller owning crate test, an explicit --test target via -ExtraCargoArgs, or pass -AllowBroadCoreLibUnitTests when this expensive lib harness is intentional."
}
$keepCoreLibTestArtifactsOnSuccess = $Package -eq "codex-core" -and
    $Lib -and
    -not [string]::IsNullOrWhiteSpace($Filter) -and
    -not $CleanCoreLibTestArtifactsOnSuccess

if ($Package -eq "codex-tui" -and -not [string]::IsNullOrWhiteSpace($Filter) -and -not $AllowBroadTuiUnitTests) {
    $hasExplicitCargoTarget = $ExtraCargoArgs -contains "--test" -or
        $ExtraCargoArgs -contains "--bin" -or
        $ExtraCargoArgs -contains "--example"
    if (-not $hasExplicitCargoTarget) {
        throw "Refusing filtered codex-tui package/unit test without -AllowBroadTuiUnitTests. Cargo compiles the full codex-tui test harness and heavy dependency graph before applying the filter. Prefer the smaller owning crate test, an explicit --test target via -ExtraCargoArgs, or pass -AllowBroadTuiUnitTests when this expensive canary is intentional."
    }
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

$testExit = Invoke-CargoWithHeartbeat -CargoArgs $cargoArgs -WorkingDirectory $codexRs -LogPath $log -LogsDirectory $logs
if ($null -eq $testExit) {
    $testExit = 1
}

if ($testExit -eq 0 -and -not $NoCleanup -and $keepCoreLibTestArtifactsOnSuccess) {
    Write-Host "Release test passed; keeping codex-core --lib test executable artifacts so repeated filtered core tests can reuse the expensive harness."
    Write-Host "Pass -CleanCoreLibTestArtifactsOnSuccess to clean them after this run."
    & powershell -NoProfile -ExecutionPolicy Bypass -File $buildScript -Mode PruneReleaseDeps
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
elseif ($testExit -eq 0 -and -not $NoCleanup) {
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
