[CmdletBinding()]
param(
    [string]$RepoRoot,
    [string]$LogPath
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path

if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $latestLog = Get-ChildItem -LiteralPath (Join-Path $RepoRoot "logs") -Filter "local-codex-build-*.log" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($latestLog) {
        $LogPath = $latestLog.FullName
    }
}

$repoNeedle = $RepoRoot.ToLowerInvariant()
$active = Get-CimInstance Win32_Process -Filter "Name = 'cargo.exe' OR Name = 'rustc.exe' OR Name = 'link.exe' OR Name = 'cmd.exe'" -ErrorAction SilentlyContinue |
    Where-Object {
        $commandLine = [string]$_.CommandLine
        $commandLine.ToLowerInvariant().Contains($repoNeedle)
    } |
    ForEach-Object {
        $commandLine = [string]$_.CommandLine
        $crate = $null
        if ($commandLine -match '--crate-name\s+([^\s]+)') {
            $crate = $matches[1]
        }
        [pscustomobject]@{
            name = $_.Name
            pid = [int]$_.ProcessId
            crate = $crate
            elapsed_min = if ($_.CreationDate) {
                [math]::Round(((Get-Date) - $_.CreationDate).TotalMinutes, 1)
            } else {
                $null
            }
        }
    }

$lines = @()
if ($LogPath -and (Test-Path -LiteralPath $LogPath)) {
    $lines = Get-Content -LiteralPath $LogPath
}

$compiled = @($lines | Select-String -Pattern '^\s+Compiling\s+([^\s]+)' | ForEach-Object { $_.Matches[0].Groups[1].Value })
$warnings = @($lines | Select-String -Pattern '^warning:')
$finished = $lines | Select-String -Pattern '^\s*Finished\s+' | Select-Object -Last 1
$errors = @($lines | Select-String -Pattern '(^error:|error\[|cargo build failed|LINK : fatal|STATUS_|out of memory)')
$lastCrate = if ($compiled.Count -gt 0) { $compiled[-1] } else { $null }
$activeCrates = @($active | Where-Object { $_.crate } | Select-Object -ExpandProperty crate -Unique)
$isLinking = [bool]($active | Where-Object { $_.name -ieq "link.exe" })
$isCargoActive = [bool]($active | Where-Object { $_.name -ieq "cargo.exe" })
$freeGb = [math]::Round(((Get-PSDrive C).Free / 1GB), 2)

$phase = "unknown"
$remaining = "unknown"
if ($finished) {
    $phase = "cargo finished; wrapper script may be verifying or deploying"
    $remaining = "usually under 2 minutes"
} elseif ($isLinking) {
    $phase = "linking codex.exe"
    $remaining = "usually 2-10 minutes on this machine"
} elseif ($activeCrates -contains "codex_cli") {
    $phase = "final codex-cli compile before link"
    $remaining = "likely 5-15 minutes, mostly compile/link/deploy"
} elseif ($activeCrates -contains "codex_tui") {
    $phase = "late TUI compile"
    $remaining = "likely 10-25 minutes"
} elseif ($isCargoActive) {
    $phase = "release compile"
    $remaining = "depends on current crate; still active"
} else {
    $phase = "no active repo-local cargo build found"
    $remaining = "none if deploy already completed; check wrapper/version"
}

[ordered]@{
    status = if ($errors.Count -gt 0) { "error_seen" } elseif ($active.Count -gt 0) { "building" } else { "idle" }
    phase = $phase
    rough_remaining = $remaining
    active_crates = $activeCrates
    active_process_count = $active.Count
    compile_lines = $compiled.Count
    last_logged_crate = $lastCrate
    warnings = $warnings.Count
    errors = $errors.Count
    free_c_drive_gb = $freeGb
    log_path = $LogPath
} | ConvertTo-Json -Depth 5
