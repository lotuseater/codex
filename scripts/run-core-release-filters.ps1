param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string[]]$Filter,

    [int]$Jobs = 1,

    [string]$LogDir = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")).Path "logs"),

    [Alias("ReuseExisting")]
    [switch]$NoBuild,

    [ValidateSet("release", "ci-test")]
    [string]$CargoProfile = "release",

    [switch]$Exact,

    [switch]$NoCapture,

    [switch]$AllowConcurrent
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$codexRs = Join-Path $repoRoot "codex-rs"
$targetProfileDir = if ($CargoProfile -eq "release") { "release" } else { $CargoProfile }
$depsDir = Join-Path $codexRs "target\$targetProfileDir\deps"

function Get-RepoRustBuildProcess {
    $escapedCodexRs = [Regex]::Escape($codexRs)
    Get-CimInstance Win32_Process |
        Where-Object {
            $_.Name -in @("cargo.exe", "rustc.exe", "link.exe", "lld-link.exe") -and
            $_.CommandLine -match $escapedCodexRs
        } |
        Select-Object ProcessId, Name, CreationDate, CommandLine
}

function Get-CoreTestBinary {
    if (-not (Test-Path -LiteralPath $depsDir)) {
        throw "Missing release deps directory: $depsDir"
    }

    $binary = Get-ChildItem -LiteralPath $depsDir -Filter "codex_core-*.exe" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    if (-not $binary) {
        throw "No codex_core release test binary found under $depsDir"
    }

    $binary.FullName
}

function ConvertTo-SafeLogName([string]$Value) {
    ($Value -replace "[^A-Za-z0-9_.-]+", "_").Trim("_")
}

function Invoke-LoggedProcess {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter(Mandatory = $true)]
        [string[]]$ArgumentList,

        [Parameter(Mandatory = $true)]
        [string]$LogPath,

        [Parameter(Mandatory = $true)]
        [string]$WorkingDirectory,

        [Parameter(Mandatory = $true)]
        [string]$Label
    )

    $stdoutLog = "$LogPath.stdout"
    $stderrLog = "$LogPath.stderr"
    Remove-Item -LiteralPath $LogPath, $stdoutLog, $stderrLog -Force -ErrorAction SilentlyContinue

    Write-Host "Running $Label"
    Write-Host ("  {0} {1}" -f $FilePath, ($ArgumentList -join " "))
    Write-Host "  log: $LogPath"

    $process = Start-Process `
        -FilePath $FilePath `
        -ArgumentList $ArgumentList `
        -WorkingDirectory $WorkingDirectory `
        -WindowStyle Hidden `
        -RedirectStandardOutput $stdoutLog `
        -RedirectStandardError $stderrLog `
        -PassThru

    while (-not $process.HasExited) {
        Start-Sleep -Seconds 30
        $process.Refresh()
        $elapsed = [DateTimeOffset]::Now - [DateTimeOffset]$process.StartTime
        Write-Host ("Still running {0} after {1:n0}s" -f $Label, $elapsed.TotalSeconds)
        if (Test-Path -LiteralPath $stderrLog) {
            Get-Content -LiteralPath $stderrLog -Tail 24
        }
        if (Test-Path -LiteralPath $stdoutLog) {
            Get-Content -LiteralPath $stdoutLog -Tail 8
        }
    }
    $process.WaitForExit()

    @(
        "=== stderr ==="
        if (Test-Path -LiteralPath $stderrLog) { Get-Content -LiteralPath $stderrLog }
        "=== stdout ==="
        if (Test-Path -LiteralPath $stdoutLog) { Get-Content -LiteralPath $stdoutLog }
    ) | Set-Content -LiteralPath $LogPath

    Get-Content -LiteralPath $LogPath -Tail 120
    $process.ExitCode
}

if (-not $AllowConcurrent) {
    $active = @(Get-RepoRustBuildProcess)
    if ($active.Count -gt 0) {
        $summary = $active |
            ForEach-Object { "$($_.Name)#$($_.ProcessId) since $($_.CreationDate)" } |
            Join-String -Separator "; "
        throw "Repo Rust build already running: $summary. Re-run after it finishes, or pass -AllowConcurrent intentionally."
    }
}

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"

Push-Location $codexRs
try {
    $oldNativeCommandPreference = $PSNativeCommandUseErrorActionPreference
    $PSNativeCommandUseErrorActionPreference = $false

    if (-not $NoBuild) {
        $buildLog = Join-Path $LogDir "codex-core-$CargoProfile-lib-no-run-$stamp.log"
        $cargoProfileArgs = if ($CargoProfile -eq "release") {
            @("--release")
        } else {
            @("--profile", $CargoProfile)
        }
        $buildExit = Invoke-LoggedProcess `
            -FilePath "cargo" `
            -ArgumentList (@("test", "-p", "codex-core") + $cargoProfileArgs + @("--lib", "--no-run", "-j", "$Jobs")) `
            -LogPath $buildLog `
            -WorkingDirectory $codexRs `
            -Label "codex-core $CargoProfile test build"
        if ($buildExit -ne 0) {
            throw "codex-core $CargoProfile lib test build failed with exit code $buildExit. Log: $buildLog"
        }
    } else {
        Write-Host "Skipping cargo build; reusing newest existing codex_core $CargoProfile test binary."
    }

    $testBinary = Get-CoreTestBinary
    Write-Host "Using $testBinary"

    foreach ($filterValue in $Filter) {
        $safeFilter = ConvertTo-SafeLogName $filterValue
        if ([string]::IsNullOrWhiteSpace($safeFilter)) {
            $safeFilter = "filter"
        }
        $runLog = Join-Path $LogDir "codex-core-$CargoProfile-lib-$safeFilter-$stamp.log"

        $args = @($filterValue)
        if ($Exact) {
            $args += "--exact"
        }
        if ($NoCapture) {
            $args += "--nocapture"
        }

        $runExit = Invoke-LoggedProcess `
            -FilePath $testBinary `
            -ArgumentList $args `
            -LogPath $runLog `
            -WorkingDirectory $codexRs `
            -Label "codex-core $CargoProfile filter '$filterValue'"
        if ($runExit -ne 0) {
            throw "codex-core $CargoProfile lib filter '$filterValue' failed with exit code $runExit. Log: $runLog"
        }
    }
}
finally {
    $PSNativeCommandUseErrorActionPreference = $oldNativeCommandPreference
    Pop-Location
}
