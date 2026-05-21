param(
    [string]$StatePath = (Join-Path $PSScriptRoot "solid_refactor_director.state.json"),
    [switch]$ScanFallback,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"

function Stop-TreeFast([int]$RootPid) {
    if ($RootPid -le 0 -or $RootPid -eq [int]$PID) {
        return @()
    }

    $process = Get-Process -Id $RootPid -ErrorAction SilentlyContinue
    if (-not $process) {
        return @()
    }

    try {
        & taskkill.exe /PID $RootPid /T /F 2>$null | Out-Null
    } catch {
        return @()
    }

    if ($LASTEXITCODE -ne 0) {
        return @()
    }

    @($RootPid)
}

$StatePath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($StatePath)
$stopped = @()

if (Test-Path -LiteralPath $StatePath) {
    try {
        $state = Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json
        if ($state.rootPid) {
            $stopped += Stop-TreeFast ([int]$state.rootPid)
        }
    } catch {
        if (-not $Quiet) {
            Write-Warning "Bad director state: $_"
        }
    }
    Remove-Item -LiteralPath $StatePath -Force -ErrorAction SilentlyContinue
}

if ($ScanFallback) {
    $fallbackRoots = Get-CimInstance Win32_Process |
        Where-Object {
            $_.ProcessId -ne [int]$PID -and
            $_.CommandLine -and
            $_.CommandLine -match "run-solid-refactor-director\.ps1"
        } |
        Select-Object -ExpandProperty ProcessId

    foreach ($rootPid in @($fallbackRoots | Sort-Object -Unique)) {
        $stopped += Stop-TreeFast ([int]$rootPid)
    }
}

if (-not $Quiet) {
    [pscustomobject]@{
        Stopped = (@($stopped | Sort-Object -Unique) -join ",")
        StatePath = $StatePath
    } | Format-List
}
