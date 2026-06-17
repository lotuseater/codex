#!/usr/bin/env pwsh
#Requires -Version 7.0
<#
.SYNOPSIS
    Alert-only watcher that detects a STALLED local Codex release build.

.DESCRIPTION
    A small, dependency-free PowerShell 7 watcher. Each iteration it:
      1. Finds repo-local build processes (rustc / cargo, optionally link.exe),
         preferring to scope by the repo path in the process path/commandline.
      2. Computes an aggregate CPU ratio = cpu-seconds / wall-seconds * 100 per
         process and takes the MAX across processes. The busiest process is the
         "is anything actually working hard" signal.
      3. Finds the newest detailed build log in -LogDir and measures its mtime age.
      4. Flags a STALL when there ARE active build processes AND the busiest one is
         nearly idle (max CPU ratio < -CpuPctThreshold) AND the log has stopped
         growing (age > -StaleLogMinutes).

    RATIONALE (why CPU-ratio + frozen log = deadlock):
        A healthy rustc/cargo burns CPU steadily, so cpu-seconds tracks wall-seconds
        (ratio ~80-90%). A real rustc DEADLOCK was once diagnosed by noticing the
        process sat alive at ~5% CPU ratio while its detailed log mtime was frozen
        (no progress, RAM/pagefile free). Neither signal alone is reliable: a slow
        single-threaded codegen unit can look idle for a moment, and an old log can
        belong to a finished build. The CONJUNCTION (idle busiest process + frozen
        log + processes still alive) is what cheaply distinguishes a deadlock from
        normal slow progress, and this watcher automates spotting it.

    This script is ALERT-ONLY. It NEVER kills or modifies any process or file and
    writes nothing but console output. Recovery (kill tree, remove .cargo-lock,
    restart) is a human/orchestrator decision -- see docs/local-build-incidents.md.

.PARAMETER LogDir
    Build-log directory, relative to the repo root (default 'logs'). The script
    lives in <repo>\scripts\, so the repo root is its parent directory.

.PARAMETER IntervalSeconds
    Seconds to sleep between iterations (default 120).

.PARAMETER CpuPctThreshold
    Max-CPU-ratio percent below which the busiest process counts as idle (default 15).

.PARAMETER StaleLogMinutes
    Log mtime age in minutes above which the log counts as frozen (default 8).

.PARAMETER MaxIterations
    0 = loop until Ctrl-C; >0 = run that many iterations then exit (default 0).

.EXAMPLE
    pwsh -File scripts\watch-codex-build-stall.ps1
    Watch forever with defaults (120s interval), printing a heartbeat each cycle.

.EXAMPLE
    pwsh -File scripts\watch-codex-build-stall.ps1 -MaxIterations 1
    Run a single dry iteration (smoke test) and exit 0.

.EXAMPLE
    pwsh -File scripts\watch-codex-build-stall.ps1 -IntervalSeconds 60 -CpuPctThreshold 10 -StaleLogMinutes 12
    Tighter cadence; only alert when very idle AND the log has been frozen 12+ min.
#>
[CmdletBinding()]
param(
    [string]$LogDir = 'logs',
    [int]$IntervalSeconds = 120,
    [double]$CpuPctThreshold = 15,
    [double]$StaleLogMinutes = 8,
    [int]$MaxIterations = 0
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Repo root = parent of the scripts\ dir this file lives in.
$RepoRoot = Split-Path -Parent $PSScriptRoot
$RepoPathToken = $RepoRoot.TrimEnd('\', '/')

# Resolve the log directory against the repo root unless an absolute path is given.
if ([System.IO.Path]::IsPathRooted($LogDir)) {
    $LogPath = $LogDir
} else {
    $LogPath = Join-Path $RepoRoot $LogDir
}

function Get-Timestamp {
    (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')
}

# Returns an array of PSCustomObjects { Pid, Name, CpuRatioPct } for active,
# repo-scoped build processes. Access-denied processes are skipped.
function Get-BuildProcessStats {
    $names = @('rustc', 'cargo', 'link')
    $procs = @()
    foreach ($n in $names) {
        try {
            $found = Get-Process -Name $n -ErrorAction SilentlyContinue
            if ($found) { $procs += $found }
        } catch {
            # Ignore lookup errors for a given name; keep scanning the rest.
        }
    }

    # Try to repo-scope by matching the repo path in the executable path. We can
    # also consult the command line via CIM. If scoping a given process is not
    # feasible (no path / access denied), fall back to keeping it.
    $cimByPid = @{}
    try {
        $cim = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match '^(rustc|cargo|link)(\.exe)?$' }
        foreach ($c in $cim) { $cimByPid[[int]$c.ProcessId] = $c }
    } catch {
        # CIM unavailable: proceed with Get-Process data only.
    }

    $now = Get-Date
    $stats = @()
    $anyScopable = $false
    $scoped = @()
    $unscopable = @()

    foreach ($p in ($procs | Sort-Object Id -Unique)) {
        $procPid = $p.Id
        $name = $p.Name

        # Determine repo-scoping from exe path and/or command line.
        $pathHint = $null
        try { $pathHint = $p.Path } catch { $pathHint = $null }
        $cmdLine = $null
        if ($cimByPid.ContainsKey($procPid)) {
            $ci = $cimByPid[$procPid]
            if (-not $pathHint) { try { $pathHint = $ci.ExecutablePath } catch {} }
            try { $cmdLine = $ci.CommandLine } catch { $cmdLine = $null }
        }

        $canScope = ($pathHint) -or ($cmdLine)
        $matchesRepo = $false
        if ($pathHint -and $pathHint -like "*$RepoPathToken*") { $matchesRepo = $true }
        if ($cmdLine -and $cmdLine -like "*$RepoPathToken*") { $matchesRepo = $true }

        # Compute CPU ratio, guarding access-denied on StartTime / CPU time.
        $cpuSeconds = $null
        try {
            $cpuSeconds = $p.TotalProcessorTime.TotalSeconds
        } catch {
            $cpuSeconds = $null
        }
        if (($null -eq $cpuSeconds) -and $cimByPid.ContainsKey($procPid)) {
            try {
                $ci2 = $cimByPid[$procPid]
                $cpuSeconds = ([double]$ci2.KernelModeTime + [double]$ci2.UserModeTime) / 1e7
            } catch {
                $cpuSeconds = $null
            }
        }

        $startTime = $null
        try { $startTime = $p.StartTime } catch { $startTime = $null }
        if (($null -eq $startTime) -and $cimByPid.ContainsKey($procPid)) {
            try { $startTime = $cimByPid[$procPid].CreationDate } catch { $startTime = $null }
        }

        if (($null -eq $cpuSeconds) -or ($null -eq $startTime)) {
            # Access-denied / unavailable timing -> skip this process entirely.
            continue
        }

        $runtimeSeconds = ($now - $startTime).TotalSeconds
        if ($runtimeSeconds -le 0) { continue }
        $cpuRatioPct = [Math]::Round(($cpuSeconds / $runtimeSeconds) * 100, 1)

        $rec = [PSCustomObject]@{
            Pid         = $procPid
            Name        = $name
            CpuRatioPct = $cpuRatioPct
        }

        if ($canScope) { $anyScopable = $true }
        if ($matchesRepo) { $scoped += $rec } else { $unscopable += $rec }
    }

    # Prefer repo-scoped processes. If we could scope at least one process and got
    # repo matches, use only those. Otherwise fall back to all rustc/cargo/link.
    if ($anyScopable -and $scoped.Count -gt 0) {
        $stats = $scoped
    } else {
        $stats = $scoped + $unscopable
    }

    return ,$stats
}

# Returns { Name; AgeMinutes } for the newest build log, or $null if none/missing.
function Get-NewestLogInfo {
    if (-not (Test-Path -LiteralPath $LogPath)) { return $null }

    $log = $null
    try {
        $log = Get-ChildItem -LiteralPath $LogPath -Filter 'local-codex-build-*.log' -File -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
    } catch {
        $log = $null
    }
    if (-not $log) {
        try {
            $log = Get-ChildItem -LiteralPath $LogPath -Filter '*.log' -File -ErrorAction SilentlyContinue |
                Sort-Object LastWriteTime -Descending | Select-Object -First 1
        } catch {
            $log = $null
        }
    }
    if (-not $log) { return $null }

    $ageMin = [Math]::Round(((Get-Date) - $log.LastWriteTime).TotalMinutes, 1)
    return [PSCustomObject]@{
        Name       = $log.Name
        AgeMinutes = $ageMin
    }
}

function Invoke-WatchIteration {
    $ts = Get-Timestamp
    $stats = Get-BuildProcessStats

    if (-not $stats -or $stats.Count -eq 0) {
        Write-Host "[watch] $ts no active build"
        return
    }

    $maxCpuRatio = ($stats | Measure-Object -Property CpuRatioPct -Maximum).Maximum
    $pids = ($stats | ForEach-Object { $_.Pid }) -join ','

    $logInfo = Get-NewestLogInfo
    if ($null -eq $logInfo) {
        $logAge = [double]::PositiveInfinity
        $logName = '(none)'
    } else {
        $logAge = $logInfo.AgeMinutes
        $logName = $logInfo.Name
    }

    $isStall = ($maxCpuRatio -lt $CpuPctThreshold) -and ($logAge -gt $StaleLogMinutes)

    if ($isStall) {
        $ageStr = if ([double]::IsInfinity($logAge)) { 'inf' } else { "$logAge" }
        Write-Host "[STALL-ALERT] $ts cpu=$maxCpuRatio% log_age=${ageStr}min log=$logName pids=$pids -> possible rustc deadlock; see docs/local-build-incidents.md"
    } else {
        $ageStr = if ([double]::IsInfinity($logAge)) { 'inf' } else { "$logAge" }
        Write-Host "[watch] $ts ok cpu=$maxCpuRatio% log_age=${ageStr}min"
    }
}

# Main loop.
$iter = 0
while ($true) {
    $iter++
    try {
        Invoke-WatchIteration
    } catch {
        # An iteration error must not crash an alert-only watcher; report and go on.
        Write-Host "[watch] $(Get-Timestamp) iteration error: $($_.Exception.Message)"
    }

    if (($MaxIterations -gt 0) -and ($iter -ge $MaxIterations)) { break }

    Start-Sleep -Seconds $IntervalSeconds
}

exit 0
