[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [ValidateSet("Status", "Diagnose", "Progress", "CleanSafe", "PruneReleaseDeps", "FastRelease", "LowMemRelease", "DevRelease", "FullRelease", "DeployOnly", "Rollback")]
    [string]$Mode = "Status",

    [string]$RepoRoot,

    [string]$SourceExe,

    [string]$WrapperDir = (Join-Path $HOME ".codex\system-wrapper"),

    [string]$BackupRoot = (Join-Path $HOME ".codex\binary-backups"),

    [string]$LocalBuildRoot = (Join-Path $HOME ".codex\local-builds"),

    [switch]$SkipDeploy,

    [switch]$SkipVerify,

    [switch]$Timings,

    [int]$Jobs = 0,

    [switch]$CleanTestArtifacts,

    [int]$CleanTestArtifactsBelowGB = 0,

    [double]$DiskRequiredGB = 5,

    [double]$DiskWarnGB = 8,

    [int]$DuplicateAuditLimit = 12,

    [switch]$UseSccache,

    [switch]$ResetReleaseCacheOnProfileChange,

    [ValidateRange(0, 2147483647)]
    [int]$TimeoutSeconds = 0
)

$ErrorActionPreference = "Stop"

if ($Mode -eq "DevRelease") {
    throw "Build only release!"
}

# Auto-enable sccache for release builds when it is available on PATH and the
# caller did not pass -UseSccache:$false explicitly. The release-only rustc
# wrapper chains sccache safely, and warm-cache hits are the single biggest
# win for partial rebuilds on this checkout.
if (-not $PSBoundParameters.ContainsKey('UseSccache') -and
    $Mode -in @("FastRelease", "LowMemRelease", "FullRelease")) {
    $autoSccache = Get-Command sccache -ErrorAction SilentlyContinue
    if ($autoSccache) {
        $UseSccache = $true
        Write-Host "sccache detected at $($autoSccache.Source) - auto-enabled. Pass -UseSccache:`$false to disable."
    }
}

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path

function Resolve-FullPath {
    param([string]$Path)

    return [System.IO.Path]::GetFullPath($Path)
}

function Assert-UnderRoot {
    param(
        [string]$Path,
        [string]$Root,
        [string]$Label
    )

    $resolvedRoot = Resolve-FullPath -Path $Root
    $resolvedPath = Resolve-FullPath -Path $Path
    $rootPrefix = $resolvedRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
        [System.IO.Path]::DirectorySeparatorChar

    if (($resolvedPath -ine $resolvedRoot) -and
        (-not $resolvedPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase))) {
        throw "$Label resolves outside expected root: $resolvedPath (root: $resolvedRoot)"
    }

    return $resolvedPath
}

function Read-JsonObject {
    param([string]$Path)

    $json = Get-Content -LiteralPath $Path -Raw
    if ([string]::IsNullOrWhiteSpace($json)) {
        return [ordered]@{}
    }

    $parsed = $json | ConvertFrom-Json
    $result = [ordered]@{}
    foreach ($property in $parsed.PSObject.Properties) {
        $result[$property.Name] = $property.Value
    }
    return $result
}

function Write-JsonObject {
    param(
        [string]$Path,
        [System.Collections.IDictionary]$Payload
    )

    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, ($Payload | ConvertTo-Json -Depth 8), $utf8NoBom)
}

function Get-WrapperEnvPath {
    param([string]$Dir)

    $path = Join-Path $Dir "system.codex-wrapper.env.json"
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Wrapper env JSON not found: $path"
    }
    return (Resolve-Path -LiteralPath $path).Path
}

function Find-VsDevCmd {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path -LiteralPath $vswhere) {
        $installPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if (-not [string]::IsNullOrWhiteSpace($installPath)) {
            $candidate = Join-Path $installPath "Common7\Tools\VsDevCmd.bat"
            if (Test-Path -LiteralPath $candidate) {
                return $candidate
            }
        }
    }

    $roots = @(
        (Join-Path $env:ProgramFiles "Microsoft Visual Studio"),
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio")
    )
    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root)) {
            continue
        }

        $candidate = Get-ChildItem -LiteralPath $root -Recurse -Filter VsDevCmd.bat -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($candidate) {
            return $candidate.FullName
        }
    }

    return $null
}

function Join-CommandLine {
    param([string[]]$CommandArgs)

    return ($CommandArgs | ForEach-Object {
        if ($_ -match "\s") {
            '"' + ($_ -replace '"', '\"') + '"'
        }
        else {
            $_
        }
    }) -join " "
}

function Get-PageFileSnapshot {
    $os = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
    $pageFiles = @(Get-CimInstance Win32_PageFileUsage -ErrorAction SilentlyContinue | ForEach-Object {
            [ordered]@{
                name = $_.Name
                allocated_mb = [int]$_.AllocatedBaseSize
                current_usage_mb = [int]$_.CurrentUsage
                peak_usage_mb = [int]$_.PeakUsage
            }
        })
    return [ordered]@{
        page_files = $pageFiles
        free_ram_gb = if ($os) { [math]::Round($os.FreePhysicalMemory / 1MB, 2) } else { $null }
        total_virtual_gb = if ($os) { [math]::Round($os.TotalVirtualMemorySize / 1MB, 2) } else { $null }
        free_virtual_gb = if ($os) { [math]::Round($os.FreeVirtualMemory / 1MB, 2) } else { $null }
    }
}

function Test-ContainsText {
    param(
        [string]$Haystack,
        [string]$Needle
    )

    return (-not [string]::IsNullOrWhiteSpace($Haystack)) -and
        ($Haystack.IndexOf($Needle, [System.StringComparison]::OrdinalIgnoreCase) -ge 0)
}

function Test-ContainsPathWithBoundary {
    param(
        [string]$Haystack,
        [string]$Path
    )

    if ([string]::IsNullOrWhiteSpace($Haystack) -or [string]::IsNullOrWhiteSpace($Path)) {
        return $false
    }

    $haystackText = $Haystack.Replace("/", "\")
    $pathText = $Path.Replace("/", "\").TrimEnd("\")
    $index = $haystackText.IndexOf($pathText, [System.StringComparison]::OrdinalIgnoreCase)
    if ($index -lt 0) {
        return $false
    }

    $afterIndex = $index + $pathText.Length
    if ($afterIndex -ge $haystackText.Length) {
        return $true
    }

    $after = $haystackText[$afterIndex]
    return $after -eq "\" -or $after -eq '"' -or [char]::IsWhiteSpace($after)
}

function Get-RepoBuildProcesses {
    param([string]$Root)

    $codexRs = Join-Path $Root "codex-rs"
    $buildToolNames = @("cargo.exe", "rustc.exe", "link.exe")
    $previousWhatIfPreference = $WhatIfPreference
    $WhatIfPreference = $false
    try {
        $processes = Get-CimInstance Win32_Process -Filter "Name = 'cargo.exe' OR Name = 'rustc.exe' OR Name = 'link.exe' OR Name = 'cmd.exe'" -ErrorAction SilentlyContinue
    }
    finally {
        $WhatIfPreference = $previousWhatIfPreference
    }
    $matching = @{}

    foreach ($process in $processes) {
        $commandLine = [string]$process.CommandLine
        $isBuildTool = $buildToolNames -contains [string]$process.Name
        $isCargoShell = ([string]$process.Name -eq "cmd.exe") -and ($commandLine -match '(^|[\s"\\])cargo(\.exe)?($|[\s"])')
        $isRepoLocal = (Test-ContainsPathWithBoundary -Haystack $commandLine -Path $Root) -or
            (Test-ContainsPathWithBoundary -Haystack $commandLine -Path $codexRs)
        if (($isBuildTool -or $isCargoShell) -and $isRepoLocal) {
            $matching[[int]$process.ProcessId] = $true
            if ($process.ParentProcessId) {
                $matching[[int]$process.ParentProcessId] = $true
            }
        }
    }

    return $processes |
        Where-Object {
            $matching.ContainsKey([int]$_.ProcessId) -or
                $matching.ContainsKey([int]$_.ParentProcessId)
        } |
        Sort-Object ProcessId |
        ForEach-Object {
            [ordered]@{
                process_name = $_.Name
                process_id = [int]$_.ProcessId
                parent_process_id = [int]$_.ParentProcessId
                command_line = [string]$_.CommandLine
            }
        }
}

function Get-CodexBuildCpuRatio {
    # Computes the CPU-utilization ratio (cpu-seconds / wall-seconds) for the
    # repo-local build processes ALREADY detected by Get-RepoBuildProcesses.
    # A genuinely busy rustc/link runs near 80-100% per core; a DEADLOCKED one
    # sits at a few percent while its detailed log mtime is frozen. Surfacing
    # this ratio lets the read-only report distinguish "stalled" from "slow".
    #
    # Reuses the exact PIDs from the canonical detector (no parallel detection):
    # each PID is re-resolved via the .NET Process API to read live timing
    # (TotalProcessorTime + StartTime), which the projected CIM hashtables omit.
    # Returns:
    #   processes        @( @{ process_name; process_id; cpu_ratio_pct; runtime_min; cpu_seconds } )
    #   max_cpu_ratio_pct  highest cpu_ratio_pct across sampled processes ($null if none)
    #   sampled_count      number of processes successfully sampled
    param([object[]]$Procs)

    $results = @()
    $now = Get-Date
    foreach ($proc in @($Procs)) {
        if ($null -eq $proc) { continue }
        $procPid = [int]$proc["process_id"]
        $procName = [string]$proc["process_name"]
        if ($procPid -le 0) { continue }

        $cpuSeconds = $null
        $runtimeSeconds = $null
        $cpuRatioPct = $null
        $runtimeMin = $null

        # StartTime/TotalProcessorTime throw Access Denied for processes the
        # current token cannot open (e.g. elevated rustc) - skip those rows.
        try {
            $previousWhatIfPreference = $WhatIfPreference
            $WhatIfPreference = $false
            try {
                $osProc = Get-Process -Id $procPid -ErrorAction Stop
            }
            finally {
                $WhatIfPreference = $previousWhatIfPreference
            }
            $cpuSeconds = [double]$osProc.TotalProcessorTime.TotalSeconds
            $runtimeSeconds = [double]((New-TimeSpan -Start $osProc.StartTime -End $now).TotalSeconds)
            if ($runtimeSeconds -gt 0) {
                $cpuRatioPct = [math]::Round($cpuSeconds / $runtimeSeconds * 100, 1)
                $runtimeMin = [math]::Round($runtimeSeconds / 60, 1)
            }
        }
        catch {
            # Access Denied / process already exited - leave fields $null and
            # still emit the row so the report shows the PID was seen.
            $cpuSeconds = $null
        }

        $results += [ordered]@{
            process_name = $procName
            process_id = $procPid
            cpu_ratio_pct = $cpuRatioPct
            runtime_min = $runtimeMin
            cpu_seconds = if ($null -ne $cpuSeconds) { [math]::Round($cpuSeconds, 1) } else { $null }
        }
    }

    $ratios = @($results | ForEach-Object { $_["cpu_ratio_pct"] } | Where-Object { $null -ne $_ })
    $maxRatio = if ($ratios.Count -gt 0) { ($ratios | Measure-Object -Maximum).Maximum } else { $null }

    return [ordered]@{
        processes = @($results)
        max_cpu_ratio_pct = $maxRatio
        sampled_count = @($ratios).Count
    }
}

function Get-RecommendedJobs {
    param(
        [int]$PerJobMemoryMB = 1800,
        [int]$PerJobDiskMB = 2200,
        [int]$Floor = 1,
        [int]$Ceiling = 0
    )

    # Picks `cargo --jobs` so that:
    #   1. RAM doesn't go below 1.5 GB headroom under peak parallelism
    #   2. Free C: stays > (jobs × PerJobDiskMB) so the build can't write
    #      itself into a disk-full state mid-link (observed 2026-05-04)
    # PerJobDiskMB defaults to 2200 — the empirical peak for parallel rustc
    # codegen + .rmeta + intermediate object files in this workspace.
    $os = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
    $cpuCount = 0
    try {
        $cpuCount = [int]((Get-CimInstance Win32_Processor -ErrorAction SilentlyContinue |
            Measure-Object -Property NumberOfLogicalProcessors -Sum).Sum)
    } catch {}
    if ($cpuCount -le 0) { $cpuCount = [Environment]::ProcessorCount }
    if ($Ceiling -le 0) { $Ceiling = $cpuCount }

    $byMem = if ($os) {
        $freeMB = [int]($os.FreePhysicalMemory / 1KB)
        $headroomMB = 1500
        $usable = [math]::Max(0, $freeMB - $headroomMB)
        [math]::Floor($usable / $PerJobMemoryMB)
    } else { 2 }

    $byDisk = $cpuCount
    if ($PerJobDiskMB -gt 0) {
        $freeDiskMB = [math]::Floor((Get-PSDrive C).Free / 1MB)
        $diskHeadroomMB = 1024
        $usableDisk = [math]::Max(0, $freeDiskMB - $diskHeadroomMB)
        $byDisk = [math]::Floor($usableDisk / $PerJobDiskMB)
    }

    $picked = [math]::Min($byMem, $byDisk)
    if ($picked -lt $Floor) { $picked = $Floor }
    return [int][math]::Max($Floor, [math]::Min($Ceiling, $picked))
}

function Get-BuildPlan {
    param(
        [string]$BuildMode,
        [string]$TargetRoot,
        [int]$JobsOverride = 0
    )

    $envOverrides = [ordered]@{}
    $cargoArgs = @("build", "-p", "codex-cli", "--release", "--bin", "codex")
    $description = "local fast release build using .cargo/config.toml release profile"
    $binary = Join-Path $TargetRoot "release\codex.exe"

    switch ($BuildMode) {
        "FastRelease" {
            $description = "local fast release build (single shared release profile)"
            if ($JobsOverride -le 0) {
                $JobsOverride = Get-RecommendedJobs -PerJobMemoryMB 1000 -PerJobDiskMB 1200 -Ceiling 4
            }
        }
        "LowMemRelease" {
            $description = "local low-memory release build (same shared release profile, lower job count)"
            if ($JobsOverride -le 0) {
                $JobsOverride = Get-RecommendedJobs -PerJobMemoryMB 1300 -PerJobDiskMB 1600 -Ceiling 2
            }
        }
        "DevRelease" {
            throw "Build only release!"
        }
        "FullRelease" {
            $description = "local low-memory release build (FullRelease alias, same shared release profile)"
            if ($JobsOverride -le 0) {
                $JobsOverride = Get-RecommendedJobs -PerJobMemoryMB 1300 -PerJobDiskMB 1600 -Ceiling 2
            }
        }
    }

    if ($JobsOverride -gt 0) {
        $cargoArgs += @("--jobs", "$JobsOverride")
    }

    if ($Timings) {
        $cargoArgs += "--timings"
    }

    return [ordered]@{
        cargo_args = $cargoArgs
        env_overrides = $envOverrides
        binary = $binary
        description = $description
    }
}

# Disk-space defenses: full release builds of this workspace need headroom for
# target/release/deps libraries, intermediate .rmeta files, and final link
# scratch. Under pressure, reclaim only artifacts that do not preserve useful
# release progress. Do not auto-delete target/release/build, gn_out, or
# .fingerprint: they are technically regeneratable, but losing them can force
# expensive build-script/native rebuild work on this machine.
function Test-AndFreeDiskSpace {
    param(
        [string]$RepoRoot,
        [double]$RequiredGB = 5,
        [double]$WarnGB = 8
    )

    $codexRs = Join-Path $RepoRoot "codex-rs"
    $tgt = Join-Path $codexRs "target"
    $tgtRelease = Join-Path $codexRs "target\release"
    $reclaimable = @(
        # The shared local release lane keeps incremental=false so all release
        # builds reuse one stable artifact shape. If an override creates this
        # cache anyway, it is not useful to the deploy lane.
        @{ Path = (Join-Path $tgtRelease "incremental"); Reason = "release/incremental (shared release lane keeps incremental disabled)" },
        @{ Path = (Join-Path $tgt "debug"); Reason = "target/debug (debug builds are disabled for this checkout)" },
        @{ Path = (Join-Path $tgt "dev-small"); Reason = "target/dev-small (non-release profile artifacts)" },
        @{ Path = (Join-Path $tgt "review-check-core"); Reason = "target/review-check-core (disposable review verification artifacts)" },
        @{ Path = (Join-Path $tgt "review-check"); Reason = "target/review-check (disposable review verification artifacts)" },
        @{ Path = (Join-Path $tgt "policy-check"); Reason = "target/policy-check (disposable policy verification artifacts)" },
        @{ Path = (Join-Path $tgt "agent-policy-verify"); Reason = "target/agent-policy-verify (disposable policy verification artifacts)" }
    )

    $freeGB = [math]::Round((Get-PSDrive C).Free / 1GB, 2)
    if ($freeGB -ge $WarnGB) {
        Write-Host "Disk OK ($freeGB GB free, threshold $WarnGB GB)."
        return
    }

    Write-Host "Disk pre-check: $freeGB GB free (below warn threshold $WarnGB GB). Reclaiming..."
    $pdbCleanup = Invoke-ReleasePdbCleanup -RepoRoot $RepoRoot
    if ($pdbCleanup["reclaimed_mb"] -gt 0) {
        Write-Host ("  - reclaimed {0,7:N1} MB from release PDB files" -f $pdbCleanup["reclaimed_mb"])
    }
    $testArtifactSummary = Get-ReleaseTestArtifactSummary -RepoRoot $RepoRoot
    if ($testArtifactSummary["total_mb"] -gt 0) {
        $testCleanup = Invoke-ReleaseTestArtifactCleanup -RepoRoot $RepoRoot
        if ($testCleanup["reclaimed_mb"] -gt 0) {
            Write-Host ("  - reclaimed {0,7:N1} MB from release test executable artifacts" -f $testCleanup["reclaimed_mb"])
        }
    }
    $depsCleanup = Invoke-ReleaseDepsOrphanCleanup -RepoRoot $RepoRoot
    if ($depsCleanup["reclaimed_mb"] -gt 0) {
        Write-Host ("  - reclaimed {0,7:N1} MB from orphaned release deps artifacts" -f $depsCleanup["reclaimed_mb"])
    }

    foreach ($entry in $reclaimable) {
        if (-not (Test-Path -LiteralPath $entry.Path)) { continue }
        $sizeMB = 0
        try {
            $sizeMB = [math]::Round((Get-ChildItem -LiteralPath $entry.Path -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum / 1MB, 1)
        } catch {}
        try {
            if (Remove-GeneratedPathFast -Path $entry.Path -Action "remove $($entry.Reason)") {
                Write-Host ("  - reclaimed {0,7:N1} MB from {1}" -f $sizeMB, $entry.Reason)
            }
        } catch {
            Write-Host ("  - skip (in use): {0}" -f $entry.Path)
        }
    }

    $freeAfterGB = [math]::Round((Get-PSDrive C).Free / 1GB, 2)
    Write-Host "Disk after reclaim: $freeAfterGB GB free."
    if ($freeAfterGB -lt $RequiredGB) {
        throw "Disk space too low for safe release build: $freeAfterGB GB free (need >= $RequiredGB GB after safe auto-clean). Build only release! Keep target/release cache when possible. Free space manually before retrying; candidates to inspect: ~/.codex/sessions, ~/.codex/logs_2.sqlite, AppData/Local/Temp. Only delete target/release/build, target/release/gn_out, target/release/.fingerprint, or ~/.cargo/registry/src when you accept a slower rebuild."
    }
}

function Invoke-ReleasePdbCleanup {
    param([string]$RepoRoot)

    $release = Join-Path $RepoRoot "codex-rs\target\release"
    if (-not (Test-Path -LiteralPath $release)) {
        return [ordered]@{ removed = 0; reclaimed_mb = 0 }
    }

    $bytes = 0
    $removed = 0
    try {
        $pdbFiles = @(Get-ChildItem -LiteralPath $release -Recurse -Force -File -Filter "*.pdb" -ErrorAction Stop)
    } catch [System.Management.Automation.ItemNotFoundException] {
        return [ordered]@{ removed = 0; reclaimed_mb = 0 }
    } catch [System.IO.DirectoryNotFoundException] {
        return [ordered]@{ removed = 0; reclaimed_mb = 0 }
    }

    foreach ($file in $pdbFiles) {
        try {
            $bytes += $file.Length
            Remove-Item -LiteralPath $file.FullName -Force -ErrorAction Stop
            $removed += 1
        } catch {}
    }

    return [ordered]@{
        removed = $removed
        reclaimed_mb = [math]::Round($bytes / 1MB, 1)
    }
}

function Get-PathSizeMB {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return 0
    }
    $item = Get-Item -LiteralPath $Path -Force
    if (-not $item.PSIsContainer) {
        return [math]::Round($item.Length / 1MB, 1)
    }
    try {
        $bytes = (Get-ChildItem -LiteralPath $Path -Recurse -Force -File -ErrorAction SilentlyContinue |
            Measure-Object Length -Sum).Sum
        return [math]::Round($bytes / 1MB, 1)
    }
    catch {
        return $null
    }
}

function Remove-GeneratedPathFast {
    [CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = "None")]
    param(
        [string]$Path,
        [string]$Action = "remove generated path"
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return $false
    }
    $item = Get-Item -LiteralPath $Path -Force
    $shouldProcess = $true
    try {
        $shouldProcess = $PSCmdlet.ShouldProcess($item.FullName, $Action)
    }
    catch {
        if ($WhatIfPreference) {
            Write-Host ("What if: {0} {1}" -f $Action, $item.FullName)
            return $false
        }
    }
    if (-not $shouldProcess) {
        return $false
    }

    try {
        if ($item.PSIsContainer) {
            [System.IO.Directory]::Delete($item.FullName, $true)
        }
        else {
            [System.IO.File]::Delete($item.FullName)
        }
    }
    catch {
        $originalError = $_
        Clear-GeneratedPathReadOnlyAttributes -Path $item.FullName
        try {
            if ($item.PSIsContainer) {
                [System.IO.Directory]::Delete($item.FullName, $true)
            }
            else {
                [System.IO.File]::Delete($item.FullName)
            }
        }
        catch {
            throw $originalError
        }
    }

    return $true
}

function Clear-GeneratedPathReadOnlyAttributes {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }

    $readOnly = [System.IO.FileAttributes]::ReadOnly
    $clearReadOnly = {
        param([string]$ItemPath)

        $attributes = [System.IO.File]::GetAttributes($ItemPath)
        if (($attributes -band $readOnly) -ne 0) {
            [System.IO.File]::SetAttributes($ItemPath, ($attributes -band (-bnot $readOnly)))
        }
    }

    & $clearReadOnly $Path
    if (-not (Get-Item -LiteralPath $Path -Force).PSIsContainer) {
        return
    }

    foreach ($child in [System.IO.Directory]::EnumerateFileSystemEntries($Path, "*", [System.IO.SearchOption]::AllDirectories)) {
        & $clearReadOnly $child
    }
}

function Invoke-GeneratedPathCleanup {
    param(
        [string]$Path,
        [string]$Root,
        [string]$Reason
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return [ordered]@{
            path = $Path
            reason = $Reason
            removed = $false
            reclaimed_mb = 0
            status = "missing"
        }
    }

    $safePath = Assert-UnderRoot -Path $Path -Root $Root -Label $Reason
    $sizeMB = Get-PathSizeMB -Path $safePath
    $removed = Remove-GeneratedPathFast -Path $safePath -Action "remove $Reason"
    return [ordered]@{
        path = $safePath
        reason = $Reason
        removed = $removed
        reclaimed_mb = if ($removed) { $sizeMB } else { 0 }
        status = if ($removed) { "removed" } else { "skipped" }
    }
}

function Get-ReleaseTestArtifactSummary {
    param([string]$RepoRoot)

    $files = @(Get-ReleaseTestArtifactFiles -RepoRoot $RepoRoot)
    $exeFiles = @($files | Where-Object { $_.Extension -ieq ".exe" })
    $pdbFiles = @($files | Where-Object { $_.Extension -ieq ".pdb" })
    $sidecarFiles = @($files | Where-Object { $_.Extension -ine ".exe" })
    $deps = Join-Path $RepoRoot "codex-rs\target\release\deps"
    $totalBytes = ($files | Measure-Object Length -Sum).Sum
    $pdbBytes = ($pdbFiles | Measure-Object Length -Sum).Sum
    $sidecarBytes = ($sidecarFiles | Measure-Object Length -Sum).Sum
    if ($null -eq $totalBytes) { $totalBytes = 0 }
    if ($null -eq $pdbBytes) { $pdbBytes = 0 }
    if ($null -eq $sidecarBytes) { $sidecarBytes = 0 }

    return [ordered]@{
        count = $exeFiles.Count
        total_mb = [math]::Round($totalBytes / 1MB, 1)
        root_exe_count = 0
        deps_exe_count = @($exeFiles | Where-Object { $_.DirectoryName -ieq $deps }).Count
        matching_pdb_count = $pdbFiles.Count
        matching_pdb_mb = [math]::Round($pdbBytes / 1MB, 1)
        sidecar_count = $sidecarFiles.Count
        sidecar_mb = [math]::Round($sidecarBytes / 1MB, 1)
    }
}

function Get-ReleaseTestArtifactFiles {
    param([string]$RepoRoot)

    $deps = Join-Path $RepoRoot "codex-rs\target\release\deps"
    if (-not (Test-Path -LiteralPath $deps)) {
        return @()
    }

    $candidatePaths = New-Object System.Collections.Generic.List[string]
    foreach ($exe in @(Get-ChildItem -LiteralPath $deps -File -Filter "*.exe" -ErrorAction SilentlyContinue)) {
        $candidatePaths.Add($exe.FullName)
    }
    foreach ($object in @(Get-ChildItem -LiteralPath $deps -File -Filter "all-*.rcgu.o" -ErrorAction SilentlyContinue)) {
        $candidatePaths.Add($object.FullName)
    }

    $pathsWithSidecars = New-Object System.Collections.Generic.List[string]
    foreach ($path in $candidatePaths) {
        $pathsWithSidecars.Add($path)
        foreach ($candidate in @([System.IO.Path]::ChangeExtension($path, ".pdb"), [System.IO.Path]::ChangeExtension($path, ".d"))) {
            if (Test-Path -LiteralPath $candidate) {
                $pathsWithSidecars.Add($candidate)
            }
        }
    }

    return @(
        $pathsWithSidecars |
            Sort-Object -Unique |
            ForEach-Object { Get-Item -LiteralPath $_ -Force }
    )
}

function Invoke-ReleaseTestArtifactCleanup {
    param([string]$RepoRoot)

    $deps = Join-Path $RepoRoot "codex-rs\target\release\deps"
    if (-not (Test-Path -LiteralPath $deps)) {
        return [ordered]@{ removed = 0; reclaimed_mb = 0; status = "missing" }
    }

    $artifacts = @(Get-ReleaseTestArtifactFiles -RepoRoot $RepoRoot)
    if ($artifacts.Count -eq 0) {
        return [ordered]@{ removed = 0; reclaimed_mb = 0; status = "empty" }
    }

    $bytes = 0
    $removed = 0
    foreach ($artifact in $artifacts) {
        $safePath = Assert-UnderRoot -Path $artifact.FullName -Root $deps -Label "release test artifact"
        $item = Get-Item -LiteralPath $safePath -Force
        $bytes += $item.Length
        Remove-Item -LiteralPath $safePath -Force -ErrorAction Stop
        $removed += 1
    }

    return [ordered]@{
        removed = $removed
        reclaimed_mb = [math]::Round($bytes / 1MB, 1)
        status = "removed"
    }
}

function Invoke-ReleaseDepsOrphanCleanup {
    param([string]$RepoRoot)

    $deps = Join-Path $RepoRoot "codex-rs\target\release\deps"
    if (-not (Test-Path -LiteralPath $deps)) {
        return [ordered]@{ removed = 0; skipped = 0; reclaimed_mb = 0; status = "missing" }
    }

    return [ordered]@{
        removed = 0
        skipped = 0
        reclaimed_mb = 0
        status = "disabled: Cargo dep-info files do not identify live rlib/rmeta outputs"
    }
}

function Invoke-SafeLocalCleanup {
    param(
        [string]$RepoRoot,
        [switch]$IncludeTestArtifacts,
        [int]$TestArtifactThresholdGB = 5
    )

    $targetRoot = Assert-UnderRoot -Path (Join-Path $RepoRoot "codex-rs\target") -Root $RepoRoot -Label "target root"
    $releaseRoot = Assert-UnderRoot -Path (Join-Path $targetRoot "release") -Root $targetRoot -Label "release target"
    $beforeBytes = (Get-PSDrive C).Free
    $cleanup = @()

    $cleanup += Invoke-GeneratedPathCleanup -Path (Join-Path $targetRoot "debug") -Root $targetRoot -Reason "target/debug (debug builds are disabled for this checkout)"
    $cleanup += Invoke-GeneratedPathCleanup -Path (Join-Path $targetRoot "dev-small") -Root $targetRoot -Reason "target/dev-small (non-release profile artifacts)"
    $cleanup += Invoke-GeneratedPathCleanup -Path (Join-Path $targetRoot "review-check-core") -Root $targetRoot -Reason "target/review-check-core (disposable review verification artifacts)"
    $cleanup += Invoke-GeneratedPathCleanup -Path (Join-Path $targetRoot "review-check") -Root $targetRoot -Reason "target/review-check (disposable review verification artifacts)"
    $cleanup += Invoke-GeneratedPathCleanup -Path (Join-Path $targetRoot "policy-check") -Root $targetRoot -Reason "target/policy-check (disposable policy verification artifacts)"
    $cleanup += Invoke-GeneratedPathCleanup -Path (Join-Path $targetRoot "agent-policy-verify") -Root $targetRoot -Reason "target/agent-policy-verify (disposable policy verification artifacts)"
    $cleanup += Invoke-GeneratedPathCleanup -Path (Join-Path $releaseRoot "incremental") -Root $releaseRoot -Reason "release/incremental (shared release lane keeps incremental disabled)"

    $pdbCleanup = Invoke-ReleasePdbCleanup -RepoRoot $RepoRoot
    $testArtifactsBefore = Get-ReleaseTestArtifactSummary -RepoRoot $RepoRoot
    $testCleanup = [ordered]@{ removed = 0; reclaimed_mb = 0; status = "not_requested" }
    $freeAfterSafeGB = [math]::Round((Get-PSDrive C).Free / 1GB, 2)
    if ($IncludeTestArtifacts) {
        if ($TestArtifactThresholdGB -le 0 -or $freeAfterSafeGB -lt $TestArtifactThresholdGB) {
            $testCleanup = Invoke-ReleaseTestArtifactCleanup -RepoRoot $RepoRoot
        }
        else {
            $testCleanup = [ordered]@{
                removed = 0
                reclaimed_mb = 0
                status = "skipped_above_threshold"
                free_c_drive_gb = $freeAfterSafeGB
                threshold_gb = $TestArtifactThresholdGB
            }
        }
    }

    $afterBytes = (Get-PSDrive C).Free
    $depsCleanup = Invoke-ReleaseDepsOrphanCleanup -RepoRoot $RepoRoot
    $afterBytes = (Get-PSDrive C).Free
    return [ordered]@{
        status = "ok"
        mode = "CleanSafe"
        free_c_drive_before_gb = [math]::Round($beforeBytes / 1GB, 2)
        free_c_drive_after_gb = [math]::Round($afterBytes / 1GB, 2)
        reclaimed_mb = [math]::Round(($afterBytes - $beforeBytes) / 1MB, 1)
        generated_paths = $cleanup
        release_pdb_cleanup = $pdbCleanup
        release_deps_orphan_cleanup = $depsCleanup
        release_test_artifacts_before = $testArtifactsBefore
        release_test_artifact_cleanup = $testCleanup
    }
}

function Invoke-PostBuildDiskCleanup {
    param(
        [string]$RepoRoot,
        [string]$BuildMode = ""
    )

    # The shared local release lane keeps incremental disabled. Cargo can still
    # create this cache if an env/config override leaks in, so sweep it after a
    # successful deploy build to preserve disk headroom. Safe: it is rebuilt on
    # demand when a deliberately incremental profile uses it.
    $inc = Join-Path $RepoRoot "codex-rs\target\release\incremental"
    if (Test-Path -LiteralPath $inc) {
        try {
            $sizeMB = [math]::Round((Get-ChildItem -LiteralPath $inc -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum / 1MB, 1)
            if (Remove-GeneratedPathFast -Path $inc -Action "remove release/incremental") {
                Write-Host ("Post-build cleanup: reclaimed {0:N1} MB from release/incremental." -f $sizeMB)
            }
        } catch {
            # Non-fatal: build already succeeded.
        }
    }

    if ($BuildMode -in @("FastRelease", "LowMemRelease", "FullRelease")) {
        $pdbCleanup = Invoke-ReleasePdbCleanup -RepoRoot $RepoRoot
        if ($pdbCleanup["reclaimed_mb"] -gt 0) {
            Write-Host ("Post-build cleanup: reclaimed {0:N1} MB from release PDB files." -f $pdbCleanup["reclaimed_mb"])
        }
        $depsCleanup = Invoke-ReleaseDepsOrphanCleanup -RepoRoot $RepoRoot
        if ($depsCleanup["reclaimed_mb"] -gt 0) {
            Write-Host ("Post-build cleanup: reclaimed {0:N1} MB from orphaned release deps artifacts." -f $depsCleanup["reclaimed_mb"])
        }
    }
}

# Memory reuse between modes: artifacts in target/dev-small are useless for a
# release build, but older local runs may still have left that profile around.
# Before kicking off a release build, evict the dev-small profile dir without
# touching the shared release cache.
function Invoke-CrossModeCleanup {
    param(
        [string]$RepoRoot,
        [string]$ActiveMode
    )

    $tgt = Join-Path $RepoRoot "codex-rs\target"
    $dropTargets = @()

    if ($ActiveMode -in @("FastRelease", "LowMemRelease", "FullRelease")) {
        $dropTargets += @{ Path = (Join-Path $tgt "dev-small"); Reason = "target/dev-small (other-profile artifacts)" }
        $dropTargets += @{ Path = (Join-Path $tgt "review-check-core"); Reason = "target/review-check-core (disposable review verification artifacts)" }
        $dropTargets += @{ Path = (Join-Path $tgt "review-check"); Reason = "target/review-check (disposable review verification artifacts)" }
        $dropTargets += @{ Path = (Join-Path $tgt "policy-check"); Reason = "target/policy-check (disposable policy verification artifacts)" }
        $dropTargets += @{ Path = (Join-Path $tgt "agent-policy-verify"); Reason = "target/agent-policy-verify (disposable policy verification artifacts)" }
    }

    foreach ($entry in $dropTargets) {
        if (-not (Test-Path -LiteralPath $entry.Path)) { continue }
        try {
            $sizeMB = [math]::Round((Get-ChildItem -LiteralPath $entry.Path -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum / 1MB, 1)
            if (Remove-GeneratedPathFast -Path $entry.Path -Action "remove $($entry.Reason)") {
                Write-Host ("Cross-mode cleanup: reclaimed {0:N1} MB from {1}" -f $sizeMB, $entry.Reason)
            }
        } catch {
            Write-Host ("Cross-mode cleanup skip (in use): {0}" -f $entry.Path)
        }
    }
}

# Catch obvious wrapper-env breakage BEFORE spending 30 minutes on a build:
# WIZARD_CODEX_OPERATION_CACHE=1 with a missing bridge script means every
# tool call will quietly fail to cache. Other env-var inconsistencies surface
# the same way — broken silently mid-session. We only warn (not fail) so a
# user temporarily testing without the bridge can still build.
function Test-WrapperEnvSanity {
    param([string]$WrapperEnvPath)

    if (-not (Test-Path -LiteralPath $WrapperEnvPath)) { return }
    try {
        $env = Read-JsonObject -Path $WrapperEnvPath
    } catch {
        Write-Host "Warning: wrapper env JSON failed to parse: $($_.Exception.Message)"
        return
    }

    $cacheOn = ([string]$env["WIZARD_CODEX_OPERATION_CACHE"]).Trim().ToLowerInvariant() -in @("1", "true", "yes", "on")
    $bridge = [string]$env["WIZARD_CODEX_CACHE_BRIDGE_PY"]
    if ($cacheOn -and (-not $bridge -or -not (Test-Path -LiteralPath $bridge))) {
        Write-Host "Warning: WIZARD_CODEX_OPERATION_CACHE=1 but WIZARD_CODEX_CACHE_BRIDGE_PY is missing or unset ($bridge). Built binary will not cache MCP/shell tool calls until this is fixed."
    }

    $cacheDir = [string]$env["WIZARD_TOOL_CACHE_DIR"]
    if ($cacheDir -and -not (Test-Path -LiteralPath $cacheDir)) {
        Write-Host "Warning: WIZARD_TOOL_CACHE_DIR points at a missing directory ($cacheDir). Cache writes will fail at runtime."
    }
}

function Show-FailureLines {
    param([string]$Path)

    $matches = Select-String -Path $Path -Pattern "error:|fatal error|failed|No space|no space|insufficient disk|LINK : fatal|paging file|STATUS_|stack buffer|out of memory|memory allocation" -ErrorAction SilentlyContinue |
        Select-Object -Last 40
    if ($matches) {
        $matches | ForEach-Object { $_.Line } | Out-Host
        return
    }

    Get-Content -LiteralPath $Path -Tail 80 | Out-Host
}

function Get-DirectorySizeGB {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return 0
    }
    try {
        $bytes = (Get-ChildItem -LiteralPath $Path -Recurse -Force -File -ErrorAction SilentlyContinue |
            Measure-Object Length -Sum).Sum
        return [math]::Round($bytes / 1GB, 2)
    }
    catch {
        return $null
    }
}

function Get-ReleasePdbSizeGB {
    param([string]$RepoRoot)

    $release = Join-Path $RepoRoot "codex-rs\target\release"
    if (-not (Test-Path -LiteralPath $release)) {
        return 0
    }
    try {
        $bytes = (Get-ChildItem -LiteralPath $release -Recurse -Force -File -Filter "*.pdb" -ErrorAction SilentlyContinue |
            Measure-Object Length -Sum).Sum
        return [math]::Round($bytes / 1GB, 2)
    }
    catch {
        return $null
    }
}

function Get-ReleaseDepsDuplicateSummary {
    param(
        [string]$RepoRoot,
        [int]$Limit = 20
    )

    $deps = Join-Path $RepoRoot "codex-rs\target\release\deps"
    if (-not (Test-Path -LiteralPath $deps)) {
        return @()
    }

    $items = Get-ChildItem -LiteralPath $deps -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^(?<base>lib.+)-[0-9a-f]{16}\.(?<ext>rlib|rmeta)$' } |
        ForEach-Object {
            [pscustomobject]@{
                key = "$($matches.base).$($matches.ext)"
                length = $_.Length
                last_write_time = $_.LastWriteTime
            }
        }

    return $items |
        Group-Object key |
        Where-Object { $_.Count -gt 1 } |
        ForEach-Object {
            $group = @($_.Group)
            $old = @($group | Sort-Object last_write_time -Descending | Select-Object -Skip 1)
            [ordered]@{
                key = $_.Name
                generations = $_.Count
                total_mb = [math]::Round(($group | Measure-Object length -Sum).Sum / 1MB, 1)
                older_mb = [math]::Round(($old | Measure-Object length -Sum).Sum / 1MB, 1)
                cleanup_policy = "inspect_only; PruneReleaseDeps deletes only dep-info orphans"
                newest = ($group | Sort-Object last_write_time -Descending | Select-Object -First 1).last_write_time.ToString("o")
                oldest = ($group | Sort-Object last_write_time | Select-Object -First 1).last_write_time.ToString("o")
            }
        } |
        Sort-Object { $_["total_mb"] } -Descending |
        Select-Object -First $Limit
}

function Get-KnownDuplicateDependencyAllowlist {
    $entries = @(
        @{ Name = "windows-sys"; Reason = "windows crate ecosystem transition; active target variants can coexist" },
        @{ Name = "windows-targets"; Reason = "windows crate ecosystem transition; platform target crates follow upstream transitive versions" },
        @{ Name = "windows_aarch64_gnullvm"; Reason = "windows target crate pulled transitively by multiple windows crate generations" },
        @{ Name = "windows_aarch64_msvc"; Reason = "windows target crate pulled transitively by multiple windows crate generations" },
        @{ Name = "windows_i686_gnu"; Reason = "windows target crate pulled transitively by multiple windows crate generations" },
        @{ Name = "windows_i686_gnullvm"; Reason = "windows target crate pulled transitively by multiple windows crate generations" },
        @{ Name = "windows_i686_msvc"; Reason = "windows target crate pulled transitively by multiple windows crate generations" },
        @{ Name = "windows_x86_64_gnu"; Reason = "windows target crate pulled transitively by multiple windows crate generations" },
        @{ Name = "windows_x86_64_gnullvm"; Reason = "windows target crate pulled transitively by multiple windows crate generations" },
        @{ Name = "windows_x86_64_msvc"; Reason = "windows target crate pulled transitively by multiple windows crate generations" },
        @{ Name = "syn"; Reason = "proc-macro ecosystem still has v1/v2 transitive users" },
        @{ Name = "thiserror"; Reason = "transitive ecosystem still has v1/v2 users" },
        @{ Name = "thiserror-impl"; Reason = "transitive ecosystem still has v1/v2 users" },
        @{ Name = "schemars"; Reason = "schema ecosystem has incompatible major versions in active transitive users" },
        @{ Name = "schemars_derive"; Reason = "schema ecosystem has incompatible major versions in active transitive users" },
        @{ Name = "digest"; Reason = "crypto ecosystem major-version transition in transitive dependencies" },
        @{ Name = "block-buffer"; Reason = "crypto ecosystem major-version transition in transitive dependencies" },
        @{ Name = "crypto-common"; Reason = "crypto ecosystem major-version transition in transitive dependencies" },
        @{ Name = "hmac"; Reason = "crypto ecosystem major-version transition in transitive dependencies" },
        @{ Name = "pbkdf2"; Reason = "crypto ecosystem major-version transition in transitive dependencies" },
        @{ Name = "sha1"; Reason = "crypto ecosystem major-version transition in transitive dependencies" },
        @{ Name = "sha2"; Reason = "crypto ecosystem major-version transition in transitive dependencies" },
        @{ Name = "constant_time_eq"; Reason = "crypto/compression transitive users require incompatible versions" },
        @{ Name = "untrusted"; Reason = "TLS stack has incompatible transitive versions" },
        @{ Name = "getrandom"; Reason = "randomness ecosystem major-version transition in transitive dependencies" },
        @{ Name = "rand"; Reason = "randomness ecosystem major-version transition in transitive dependencies" },
        @{ Name = "rand_chacha"; Reason = "randomness ecosystem major-version transition in transitive dependencies" },
        @{ Name = "rand_core"; Reason = "randomness ecosystem major-version transition in transitive dependencies" },
        @{ Name = "tokio-tungstenite"; Reason = "temporary fork/upstream websocket split during remote-control merge" },
        @{ Name = "tungstenite"; Reason = "temporary fork/upstream websocket split during remote-control merge" }
    )

    $known = [ordered]@{}
    foreach ($entry in $entries) {
        $known[$entry.Name] = $entry.Reason
    }
    return $known
}

function Get-CargoLockDuplicateVersionAudit {
    param(
        [string]$RepoRoot,
        [int]$Limit = 20
    )

    $lockPath = Join-Path $RepoRoot "codex-rs\Cargo.lock"
    if (-not (Test-Path -LiteralPath $lockPath)) {
        return [ordered]@{ status = "missing"; path = $lockPath }
    }

    $packages = New-Object System.Collections.Generic.List[object]
    $current = $null
    foreach ($line in Get-Content -LiteralPath $lockPath) {
        if ($line -eq "[[package]]") {
            if ($current -and $current.Contains("name") -and $current.Contains("version")) {
                [void]$packages.Add([pscustomobject]$current)
            }
            $current = @{}
            continue
        }
        if ($null -eq $current) {
            continue
        }
        if ($line -match '^name = "(.+)"$') {
            $current["name"] = $matches[1]
            continue
        }
        if ($line -match '^version = "(.+)"$') {
            $current["version"] = $matches[1]
            continue
        }
        if ($line -match '^source = "(.+)"$') {
            $current["source"] = $matches[1]
        }
    }
    if ($current -and $current.Contains("name") -and $current.Contains("version")) {
        [void]$packages.Add([pscustomobject]$current)
    }

    $known = Get-KnownDuplicateDependencyAllowlist
    $duplicates = @(
        $packages |
            Group-Object name |
            ForEach-Object {
                $versions = @(
                    $_.Group |
                        ForEach-Object {
                            if ($_.source) { "$($_.version) [$($_.source)]" } else { $_.version }
                        } |
                        Sort-Object -Unique
                )
                if ($versions.Count -gt 1) {
                    $name = $_.Name
                    $reason = if ($known.Contains($name)) { [string]$known[$name] } else { $null }
                    [pscustomobject]@{
                        name = $name
                        count = $versions.Count
                        versions = $versions
                        classification = if ($reason) { "known_unavoidable" } else { "action_required" }
                        reason = $reason
                    }
                }
            }
    )

    $actionRequired = @($duplicates | Where-Object { $_.classification -eq "action_required" } | Sort-Object name)
    $knownUnavoidable = @($duplicates | Where-Object { $_.classification -eq "known_unavoidable" } | Sort-Object name)
    $limitedActionRequired = if ($Limit -le 0) { $actionRequired } else { @($actionRequired | Select-Object -First $Limit) }
    $limitedKnownUnavoidable = if ($Limit -le 0) { $knownUnavoidable } else { @($knownUnavoidable | Select-Object -First $Limit) }

    return [ordered]@{
        status = "ok"
        path = $lockPath
        duplicate_package_names = $duplicates.Count
        action_required_count = $actionRequired.Count
        known_unavoidable_count = $knownUnavoidable.Count
        action_required = @($limitedActionRequired)
        known_unavoidable_sample = @($limitedKnownUnavoidable)
        known_allowlist_names = @($known.Keys | Sort-Object)
    }
}

function Get-ReleaseProfileStampPath {
    param([string]$RepoRoot)

    return Join-Path $RepoRoot "codex-rs\target\release\.codex-local-release-profile.json"
}

function Get-ReleaseProfileSignature {
    param([string]$RepoRoot)

    $codexRs = Join-Path $RepoRoot "codex-rs"
    $configPath = Join-Path $codexRs ".cargo\config.toml"
    $files = @()
    foreach ($path in @($configPath)) {
        $sha256 = "missing"
        if (Test-Path -LiteralPath $path) {
            $sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        }
        $files += [ordered]@{
            path = $path
            sha256 = $sha256
        }
    }

    $rustcVersion = "rustc-unavailable"
    try {
        $rustcVersion = (& rustc -Vv 2>$null) -join "`n"
    }
    catch {}

    $details = [ordered]@{
        release_lane = "shared-low-memory-release-v1"
        files = $files
        rustc_version = $rustcVersion
    }
    $json = $details | ConvertTo-Json -Depth 8 -Compress
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hashBytes = $sha.ComputeHash($bytes)
    }
    finally {
        $sha.Dispose()
    }

    return [ordered]@{
        signature = (-join ($hashBytes | ForEach-Object { $_.ToString("x2") }))
        details = $details
    }
}

function Get-ReleaseProfileState {
    param([string]$RepoRoot)

    $stampPath = Get-ReleaseProfileStampPath -RepoRoot $RepoRoot
    $current = Get-ReleaseProfileSignature -RepoRoot $RepoRoot
    $stamp = $null
    if (Test-Path -LiteralPath $stampPath) {
        try {
            $stamp = Read-JsonObject -Path $stampPath
        }
        catch {
            $stamp = [ordered]@{
                read_error = $_.Exception.Message
            }
        }
    }

    $stampSignature = if ($stamp -and $stamp.Contains("signature")) { [string]$stamp["signature"] } else { $null }
    return [ordered]@{
        stamp_path = $stampPath
        stamp_exists = [bool]$stamp
        current_signature = [string]$current["signature"]
        stamp_signature = $stampSignature
        matches = (-not $stamp) -or ($stampSignature -eq [string]$current["signature"])
        stamped_at = if ($stamp -and $stamp.Contains("stamped_at")) { [string]$stamp["stamped_at"] } else { $null }
        mode = if ($stamp -and $stamp.Contains("mode")) { [string]$stamp["mode"] } else { $null }
        read_error = if ($stamp -and $stamp.Contains("read_error")) { [string]$stamp["read_error"] } else { $null }
    }
}

function Write-ReleaseProfileStamp {
    param(
        [string]$RepoRoot,
        [string]$ModeName,
        [string[]]$CargoArgs
    )

    $stampPath = Get-ReleaseProfileStampPath -RepoRoot $RepoRoot
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $stampPath) | Out-Null
    $signature = Get-ReleaseProfileSignature -RepoRoot $RepoRoot
    Write-JsonObject -Path $stampPath -Payload ([ordered]@{
            stamped_at = (Get-Date).ToString("o")
            mode = $ModeName
            cargo_args = $CargoArgs
            signature = [string]$signature["signature"]
            details = $signature["details"]
        })
    return $stampPath
}

function Invoke-ReleaseProfileCacheReset {
    param(
        [string]$RepoRoot,
        [System.Collections.IDictionary]$ProfileState
    )

    $targetRoot = Assert-UnderRoot -Path (Join-Path $RepoRoot "codex-rs\target") -Root $RepoRoot -Label "target root"
    $releaseRoot = Assert-UnderRoot -Path (Join-Path $targetRoot "release") -Root $targetRoot -Label "release target"
    if (-not (Test-Path -LiteralPath $releaseRoot)) {
        return [ordered]@{
            removed = $false
            reclaimed_mb = 0
            status = "missing"
            profile_state = $ProfileState
        }
    }

    $sizeMB = Get-PathSizeMB -Path $releaseRoot
    $removed = Remove-GeneratedPathFast -Path $releaseRoot -Action "remove release target cache"
    return [ordered]@{
        removed = $removed
        reclaimed_mb = if ($removed) { $sizeMB } else { 0 }
        status = if ($removed) { "removed" } else { "skipped" }
        profile_state = $ProfileState
    }
}

function Write-BuildLogEvent {
    param(
        [string]$Path,
        [string]$Phase,
        [System.Collections.IDictionary]$Payload
    )

    $entry = [ordered]@{
        phase = $Phase
        timestamp = (Get-Date).ToString("o")
        data = $Payload
    }
    Add-Content -LiteralPath $Path -Value ""
    Add-Content -LiteralPath $Path -Value "### build-local-codex:$Phase"
    Add-Content -LiteralPath $Path -Value ($entry | ConvertTo-Json -Depth 8)
}

function Initialize-BuildLog {
    param(
        [string]$Root,
        [string]$Path,
        [string[]]$CargoArgs,
        [System.Collections.IDictionary]$EnvOverrides
    )

    $codexRs = Join-Path $Root "codex-rs"
    $releaseTarget = Join-Path $codexRs "target\release"
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
    Set-Content -LiteralPath $Path -Encoding UTF8 -Value "### build-local-codex:start"
    Write-BuildLogEvent -Path $Path -Phase "environment" -Payload ([ordered]@{
            repo_root = $Root
            codex_rs = $codexRs
            cargo_args = $CargoArgs
            env_overrides = $EnvOverrides
            release_target_gb = Get-DirectorySizeGB -Path $releaseTarget
            debug_target_gb = Get-DirectorySizeGB -Path (Join-Path $codexRs "target\debug")
            free_c_drive_gb = [math]::Round((Get-PSDrive C).Free / 1GB, 2)
            memory = Get-PageFileSnapshot
            release_profile_state = Get-ReleaseProfileState -RepoRoot $Root
            active_build_processes = @(Get-RepoBuildProcesses -Root $Root)
        })
}

function Ensure-ReleaseOnlyRustcWrapper {
    param([string]$Root)

    if ([System.IO.Path]::DirectorySeparatorChar -ne "\") {
        return
    }

    $source = Join-Path $Root "scripts\cargo-release-only-rustc-wrapper.rs"
    $exe = Join-Path $Root "scripts\cargo-release-only-rustc-wrapper.exe"
    if (-not (Test-Path -LiteralPath $source)) {
        throw "Build only release! rustc wrapper source not found: $source"
    }

    $needsBuild = -not (Test-Path -LiteralPath $exe)
    if (-not $needsBuild) {
        $needsBuild = (Get-Item -LiteralPath $source).LastWriteTimeUtc -gt
            (Get-Item -LiteralPath $exe).LastWriteTimeUtc
    }
    if (-not $needsBuild) {
        return
    }

    & rustc $source -O -o $exe
    if ($LASTEXITCODE -ne 0) {
        throw "Build only release! failed to compile rustc wrapper."
    }
}

function Invoke-CmdWithOptionalTimeout {
    param(
        [string]$CommandLine,
        [ValidateRange(0, 2147483647)]
        [int]$TimeoutSeconds,
        [string]$LogPath,
        [string]$Root
    )

    if ($TimeoutSeconds -le 0) {
        & cmd.exe /d /s /c $CommandLine
        return $LASTEXITCODE
    }

    $timeoutMs = [Math]::Min([int64]$TimeoutSeconds * 1000L, [int]::MaxValue)
    $cmdExe = if ($env:ComSpec) { $env:ComSpec } else { "cmd.exe" }
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $cmdExe
    $startInfo.Arguments = "/d /s /c $CommandLine"
    $startInfo.WorkingDirectory = (Get-Location).ProviderPath
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true

    $process = [System.Diagnostics.Process]::Start($startInfo)
    try {
        if ($process.WaitForExit([int]$timeoutMs)) {
            return $process.ExitCode
        }

        Write-Warning "Build command exceeded TimeoutSeconds=$TimeoutSeconds; stopping process tree rooted at PID $($process.Id)."
        Write-BuildLogEvent -Path $LogPath -Phase "cargo-timeout" -Payload ([ordered]@{
                timeout_seconds = $TimeoutSeconds
                command = $CommandLine
                process_id = $process.Id
                memory = Get-PageFileSnapshot
                active_build_processes = @(Get-RepoBuildProcesses -Root $Root)
            })

        & taskkill.exe /PID $process.Id /T /F *> $null
        $process.WaitForExit(10000) | Out-Null
        return 124
    }
    finally {
        if ($process) {
            $process.Dispose()
        }
    }
}

function Invoke-CodexBuild {
    param(
        [string]$Root,
        [string[]]$CargoArgs,
        [System.Collections.IDictionary]$EnvOverrides,
        [string]$LogPath,
        [int]$TimeoutSeconds = 0
    )

    $oldPath = $env:Path
    $cargoBin = Join-Path $HOME ".cargo\bin"
    if ((Test-Path -LiteralPath $cargoBin) -and -not $env:Path.Contains($cargoBin)) {
        $env:Path = "$cargoBin;$env:Path"
    }

    $previousEnv = @{}
    foreach ($key in $EnvOverrides.Keys) {
        $previousEnv[$key] = [Environment]::GetEnvironmentVariable($key, "Process")
        [Environment]::SetEnvironmentVariable($key, [string]$EnvOverrides[$key], "Process")
    }

    Initialize-BuildLog -Root $Root -Path $LogPath -CargoArgs $CargoArgs -EnvOverrides $EnvOverrides
    Push-Location (Join-Path $Root "codex-rs")
    $nativeCommandPreference = Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue
    $oldNativeCommandPreference = $null
    $oldErrorActionPreference = $ErrorActionPreference
    if ($nativeCommandPreference) {
        $oldNativeCommandPreference = $PSNativeCommandUseErrorActionPreference
        $PSNativeCommandUseErrorActionPreference = $false
    }
    $ErrorActionPreference = "Continue"
    try {
        $link = Get-Command link.exe -ErrorAction SilentlyContinue
        if ($link) {
            $cargoLine = "cargo " + (Join-CommandLine -CommandArgs $CargoArgs)
            Write-BuildLogEvent -Path $LogPath -Phase "cargo-start" -Payload ([ordered]@{
                    command = $cargoLine
                    linker = $link.Source
                })
            $cmdLine = "$cargoLine >> `"$LogPath`" 2>&1"
            $exitCode = Invoke-CmdWithOptionalTimeout -CommandLine $cmdLine -TimeoutSeconds $TimeoutSeconds -LogPath $LogPath -Root $Root
            Write-BuildLogEvent -Path $LogPath -Phase "cargo-exit" -Payload ([ordered]@{
                    exit_code = $exitCode
                    memory = Get-PageFileSnapshot
                    active_build_processes = @(Get-RepoBuildProcesses -Root $Root)
                })
            return $exitCode
        }

        $vsDevCmd = Find-VsDevCmd
        if ([string]::IsNullOrWhiteSpace($vsDevCmd)) {
            throw "MSVC linker link.exe is not on PATH and VsDevCmd.bat was not found. Install Visual Studio Build Tools with the C++ workload."
        }

        $cargoLine = "cargo " + (Join-CommandLine -CommandArgs $CargoArgs)
        Write-BuildLogEvent -Path $LogPath -Phase "cargo-start" -Payload ([ordered]@{
                command = $cargoLine
                vs_dev_cmd = $vsDevCmd
            })
        $cmdLine = "call `"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && $cargoLine >> `"$LogPath`" 2>&1"
        $exitCode = Invoke-CmdWithOptionalTimeout -CommandLine $cmdLine -TimeoutSeconds $TimeoutSeconds -LogPath $LogPath -Root $Root
        Write-BuildLogEvent -Path $LogPath -Phase "cargo-exit" -Payload ([ordered]@{
                exit_code = $exitCode
                memory = Get-PageFileSnapshot
                active_build_processes = @(Get-RepoBuildProcesses -Root $Root)
            })
        return $exitCode
    }
    finally {
        Pop-Location
        $ErrorActionPreference = $oldErrorActionPreference
        if ($nativeCommandPreference) {
            $PSNativeCommandUseErrorActionPreference = $oldNativeCommandPreference
        }
        foreach ($key in $EnvOverrides.Keys) {
            [Environment]::SetEnvironmentVariable($key, $previousEnv[$key], "Process")
        }
        $env:Path = $oldPath
    }
}

function Invoke-Deploy {
    param(
        [string]$ExePath,
        [string]$ModeName,
        [string]$BuildStamp = ""
    )

    $source = (Resolve-Path -LiteralPath $ExePath).Path
    $envPath = Get-WrapperEnvPath -Dir $WrapperDir
    $payload = Read-JsonObject -Path $envPath
    $previousRealExe = [string]$payload["WIZARD_CODEX_REAL_EXE"]

    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $installDir = Join-Path $LocalBuildRoot "codex-custom-$stamp"
    $activeExe = Join-Path $installDir "codex.exe"
    $backupDir = Join-Path $BackupRoot "build-local-codex-$stamp"
    $envBackup = Join-Path $backupDir "system.codex-wrapper.env.json.bak"
    $manifestPath = Join-Path $backupDir "manifest.json"

    if ($PSCmdlet.ShouldProcess($activeExe, "copy verified Codex binary from $source")) {
        New-Item -ItemType Directory -Force -Path $installDir | Out-Null
        New-Item -ItemType Directory -Force -Path $backupDir | Out-Null
        Copy-Item -LiteralPath $source -Destination $activeExe -Force
        Copy-Item -LiteralPath $envPath -Destination $envBackup -Force

        Write-JsonObject -Path $manifestPath -Payload ([ordered]@{
            created_at = (Get-Date).ToString("o")
            action = "build-local-codex"
            mode = $ModeName
            local_build_stamp = $BuildStamp
            source_exe = $source
            active_exe = $activeExe
            wrapper_env_path = $envPath
            wrapper_env_backup = $envBackup
            previous_real_exe = $previousRealExe
        })
        $payload["WIZARD_CODEX_REAL_EXE"] = $activeExe
        $payload["WIZARD_CODEX_LOCAL_FORK_MANIFEST"] = $manifestPath
        $payload["WIZARD_CODEX_LOCAL_FORK_INSTALLED_AT"] = (Get-Date).ToString("o")
        if (-not [string]::IsNullOrWhiteSpace($BuildStamp)) {
            $payload["WIZARD_CODEX_LOCAL_BUILD_STAMP"] = $BuildStamp
        }
        Write-JsonObject -Path $envPath -Payload $payload
    }

    if (-not $SkipVerify -and (Test-Path -LiteralPath $activeExe)) {
        $versionOutput = & $activeExe --version
        $versionOutput | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "Copied Codex exe failed --version with exit code $LASTEXITCODE"
        }
        if (-not [string]::IsNullOrWhiteSpace($BuildStamp) -and $versionOutput -notmatch [regex]::Escape($BuildStamp)) {
            throw "Copied Codex exe --version did not include local build stamp $BuildStamp"
        }
    }

    return [ordered]@{
        active_exe = $activeExe
        wrapper_env_path = $envPath
        backup_manifest = $manifestPath
        previous_real_exe = $previousRealExe
    }
}

function Invoke-Rollback {
    $envPath = Get-WrapperEnvPath -Dir $WrapperDir
    $manifests = @()
    if (Test-Path -LiteralPath $BackupRoot) {
        $manifests = Get-ChildItem -LiteralPath $BackupRoot -Filter manifest.json -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -like "*build-local-codex-*" } |
            Sort-Object LastWriteTime -Descending
    }

    if ($manifests) {
        $manifest = Read-JsonObject -Path $manifests[0].FullName
        $envBackup = [string]$manifest["wrapper_env_backup"]
        if (-not (Test-Path -LiteralPath $envBackup)) {
            throw "Rollback env backup does not exist: $envBackup"
        }

        if ($PSCmdlet.ShouldProcess($envPath, "restore wrapper env from $envBackup")) {
            Copy-Item -LiteralPath $envBackup -Destination $envPath -Force
        }
    }
    else {
        $payload = Read-JsonObject -Path $envPath
        $standardExe = [string]$payload["WIZARD_CODEX_STANDARD_NPM_NATIVE_EXE"]
        if ([string]::IsNullOrWhiteSpace($standardExe)) {
            throw "No build-local-codex rollback manifest found and WIZARD_CODEX_STANDARD_NPM_NATIVE_EXE is missing."
        }
        if (-not (Test-Path -LiteralPath $standardExe)) {
            throw "Standard Codex exe does not exist: $standardExe"
        }

        $payload["WIZARD_CODEX_REAL_EXE"] = $standardExe
        $payload["WIZARD_CODEX_LOCAL_FORK_RESTORED_AT"] = (Get-Date).ToString("o")
        if ($PSCmdlet.ShouldProcess($envPath, "restore WIZARD_CODEX_REAL_EXE to standard exe $standardExe")) {
            Write-JsonObject -Path $envPath -Payload $payload
        }
    }

    $payload = Read-JsonObject -Path $envPath
    $realExe = [string]$payload["WIZARD_CODEX_REAL_EXE"]
    if (-not $SkipVerify) {
        & $realExe --version | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "Rollback Codex exe failed --version with exit code $LASTEXITCODE"
        }
    }

    return [ordered]@{
        status = "ok"
        wrapper_env_path = $envPath
        real_exe = $realExe
    }
}

$targetRoot = Assert-UnderRoot -Path (Join-Path $RepoRoot "codex-rs\target") -Root $RepoRoot -Label "target root"
$releaseBinary = Assert-UnderRoot -Path (Join-Path $targetRoot "release\codex.exe") -Root $targetRoot -Label "release binary"
$activeBuilds = @(Get-RepoBuildProcesses -Root $RepoRoot)

# CPU-utilization ratio for the active build processes (deadlock signal).
# Computed once here so every read-only report below can reuse it. The
# stall_suspect flag combines a low max CPU ratio with a frozen newest log:
# a busy build runs near 80-100%/core, a deadlocked rustc sits at a few
# percent while its detailed log mtime stops advancing.
$cpuRatioReport = Get-CodexBuildCpuRatio -Procs $activeBuilds
$newestBuildLog = $null
$buildLogDir = Join-Path $RepoRoot "logs"
if (Test-Path -LiteralPath $buildLogDir) {
    $newestBuildLog = Get-ChildItem -LiteralPath $buildLogDir -Filter "local-codex-build-*.log" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
}
$buildLogStaleMin = if ($newestBuildLog) {
    [math]::Round((New-TimeSpan -Start $newestBuildLog.LastWriteTime -End (Get-Date)).TotalMinutes, 1)
} else { $null }
$stallSuspect = $false
if ($activeBuilds.Count -gt 0 -and
    $null -ne $cpuRatioReport["max_cpu_ratio_pct"] -and
    [double]$cpuRatioReport["max_cpu_ratio_pct"] -lt 15 -and
    $null -ne $buildLogStaleMin -and
    [double]$buildLogStaleMin -gt 8) {
    $stallSuspect = $true
}

$envPath = if (Test-Path -LiteralPath (Join-Path $WrapperDir "system.codex-wrapper.env.json")) {
    Get-WrapperEnvPath -Dir $WrapperDir
}
else {
    $null
}
$wrapperPayload = if ($envPath) { Read-JsonObject -Path $envPath } else { [ordered]@{} }

function Get-RecentBuildLogSummaries {
    param(
        [string]$Root,
        [int]$Limit = 8
    )

    $logRoot = Join-Path $Root "logs"
    if (-not (Test-Path -LiteralPath $logRoot)) {
        return @()
    }

    return Get-ChildItem -LiteralPath $logRoot -Filter "*.log" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First $Limit |
        ForEach-Object {
            $errors = Select-String -Path $_.FullName -Pattern "error\[|^error: |LLVM ERROR|fatal error|STATUS_|paging file|os error 112|os error 1455|out of memory|memory allocation|test result: FAILED|could not compile" -ErrorAction SilentlyContinue
            $finished = Select-String -Path $_.FullName -Pattern "^\s*Finished |test result: ok" -ErrorAction SilentlyContinue | Select-Object -Last 1
            [ordered]@{
                path = $_.FullName
                size_kb = [math]::Round($_.Length / 1KB, 1)
                last_write_time = $_.LastWriteTime.ToString("o")
                error_count = @($errors).Count
                first_error = if ($errors) { @($errors)[0].Line.Trim() } else { $null }
                last_success = if ($finished) { $finished.Line.Trim() } else { $null }
            }
        }
}

if ($Mode -in @("Status", "Diagnose")) {
    $statusPayload = [ordered]@{
        status = "ok"
        mode = $Mode
        repo_root = $RepoRoot
        active_build_processes = $activeBuilds
        build_cpu_ratio = $cpuRatioReport
        max_cpu_ratio_pct = $cpuRatioReport["max_cpu_ratio_pct"]
        newest_build_log_stale_min = $buildLogStaleMin
        stall_suspect = $stallSuspect
        release_binary = if (Test-Path -LiteralPath $releaseBinary) {
            $item = Get-Item -LiteralPath $releaseBinary
            [ordered]@{
                path = $item.FullName
                length = $item.Length
                last_write_time = $item.LastWriteTime.ToString("o")
            }
        } else { $null }
        wrapper_env_path = $envPath
        wrapper_real_exe = [string]$wrapperPayload["WIZARD_CODEX_REAL_EXE"]
        release_profile_state = Get-ReleaseProfileState -RepoRoot $RepoRoot
        free_c_drive_bytes = (Get-PSDrive C).Free
    }
    if ($Mode -eq "Diagnose") {
        $statusPayload["target_release_gb"] = Get-DirectorySizeGB -Path (Join-Path $RepoRoot "codex-rs\target\release")
        $statusPayload["target_debug_gb"] = Get-DirectorySizeGB -Path (Join-Path $RepoRoot "codex-rs\target\debug")
        $statusPayload["target_dev_small_gb"] = Get-DirectorySizeGB -Path (Join-Path $RepoRoot "codex-rs\target\dev-small")
        $statusPayload["memory"] = Get-PageFileSnapshot
        $statusPayload["release_deps_prune"] = "disabled: Cargo dep-info files do not identify live rlib/rmeta outputs"
        $statusPayload["release_pdb_gb"] = Get-ReleasePdbSizeGB -RepoRoot $RepoRoot
        $statusPayload["release_test_artifacts"] = Get-ReleaseTestArtifactSummary -RepoRoot $RepoRoot
        $statusPayload["release_deps_duplicate_summary"] = @(Get-ReleaseDepsDuplicateSummary -RepoRoot $RepoRoot -Limit 12)
        $statusPayload["cargo_lock_duplicate_versions"] = Get-CargoLockDuplicateVersionAudit -RepoRoot $RepoRoot -Limit $DuplicateAuditLimit
        $statusPayload["recent_logs"] = @(Get-RecentBuildLogSummaries -Root $RepoRoot)
    }
    $statusPayload | ConvertTo-Json -Depth 8
    return
}

if ($Mode -eq "Progress") {
    # One-shot build progress probe. Designed for callers (Claude sessions,
    # status dashboards, CI watchers) that want a compact JSON snapshot
    # instead of tailing the log themselves.
    $logRoot = Join-Path $RepoRoot "logs"
    $log = if (Test-Path -LiteralPath $logRoot) {
        Get-ChildItem -LiteralPath $logRoot -Filter "local-codex-build-*.log" -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
    } else { $null }
    $rustcInfo = $null
    $rustcProc = Get-CimInstance Win32_Process -Filter "Name='rustc.exe'" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($rustcProc) {
        $crate = "?"
        if ($rustcProc.CommandLine -match '--crate-name (\S+)') { $crate = $matches[1] }
        $rustcInfo = [ordered]@{
            pid = [int]$rustcProc.ProcessId
            crate = $crate
            working_set_mb = [math]::Round($rustcProc.WorkingSetSize / 1MB, 1)
            elapsed_min = [math]::Round((New-TimeSpan -Start $rustcProc.CreationDate -End (Get-Date)).TotalMinutes, 1)
        }
    }
    $logSummary = $null
    if ($log) {
        $compileCount = (Select-String -Path $log.FullName -Pattern '^\s*Compiling ' -ErrorAction SilentlyContinue).Count
        $codexCrates = Select-String -Path $log.FullName -Pattern '^\s*Compiling codex-' -ErrorAction SilentlyContinue |
            ForEach-Object { (($_.Line -split '\s+Compiling\s+', 2)[1] -split ' ', 2)[0] }
        $finished = (Select-String -Path $log.FullName -Pattern '^\s*Finished ' -ErrorAction SilentlyContinue) | Select-Object -Last 1
        $errors = Select-String -Path $log.FullName -Pattern 'error\[|^error: |LLVM ERROR|fatal error|STATUS_|cargo build failed|LINK : fatal|out of memory' -ErrorAction SilentlyContinue
        $logSummary = [ordered]@{
            log_path = $log.FullName
            log_size_kb = [math]::Round($log.Length / 1KB, 1)
            log_last_write = $log.LastWriteTime.ToString("o")
            compile_count = $compileCount
            codex_crates_compiled = @($codexCrates).Count
            last_codex_crate = if ($codexCrates) { @($codexCrates)[-1] } else { $null }
            finished_line = if ($finished) { $finished.Line.Trim() } else { $null }
            error_count = @($errors).Count
            first_error = if ($errors) { @($errors)[0].Line.Trim() } else { $null }
        }
    }
    # CPU-ratio detail for the active build processes + stall_suspect flag,
    # so a Progress snapshot can tell a deadlocked rustc from a busy one.
    $rustcCpuRatioPct = $null
    if ($rustcProc) {
        $rustcRow = @($cpuRatioReport["processes"]) |
            Where-Object { $_["process_id"] -eq [int]$rustcProc.ProcessId } |
            Select-Object -First 1
        if ($rustcRow) { $rustcCpuRatioPct = $rustcRow["cpu_ratio_pct"] }
    }
    [ordered]@{
        status = "ok"
        repo_root = $RepoRoot
        active_build_processes = $activeBuilds.Count
        rustc = $rustcInfo
        rustc_cpu_ratio_pct = $rustcCpuRatioPct
        build_cpu_ratio = $cpuRatioReport
        max_cpu_ratio_pct = $cpuRatioReport["max_cpu_ratio_pct"]
        newest_build_log_stale_min = $buildLogStaleMin
        stall_suspect = $stallSuspect
        log = $logSummary
        free_c_drive_gb = [math]::Round((Get-PSDrive C).Free / 1GB, 1)
        memory = Get-PageFileSnapshot
    } | ConvertTo-Json -Depth 8
    return
}

if ($Mode -eq "CleanSafe") {
    if ($activeBuilds.Count -gt 0) {
        $ids = ($activeBuilds | ForEach-Object { $_["process_id"] }) -join ", "
        throw "Repo-local cargo/rustc build process already active ($ids). Run Status to inspect it; CleanSafe will not remove artifacts while a build is active."
    }
    Invoke-SafeLocalCleanup -RepoRoot $RepoRoot -IncludeTestArtifacts:$CleanTestArtifacts -TestArtifactThresholdGB $CleanTestArtifactsBelowGB |
        ConvertTo-Json -Depth 8
    return
}

if ($Mode -eq "PruneReleaseDeps") {
    if ($activeBuilds.Count -gt 0) {
        $ids = ($activeBuilds | ForEach-Object { $_["process_id"] }) -join ", "
        throw "Repo-local cargo/rustc build process already active ($ids). Run Status to inspect it; PruneReleaseDeps will not remove artifacts while a build is active."
    }
    [ordered]@{
        status = "ok"
        mode = "PruneReleaseDeps"
        release_deps_orphan_cleanup = Invoke-ReleaseDepsOrphanCleanup -RepoRoot $RepoRoot
        free_c_drive_gb = [math]::Round((Get-PSDrive C).Free / 1GB, 2)
    } | ConvertTo-Json -Depth 8
    return
}

if ($Mode -eq "Rollback") {
    Invoke-Rollback | ConvertTo-Json -Depth 6
    return
}

if ($Mode -eq "DeployOnly") {
    if ([string]::IsNullOrWhiteSpace($SourceExe)) {
        $SourceExe = $releaseBinary
    }
    Invoke-Deploy -ExePath $SourceExe -ModeName $Mode | ConvertTo-Json -Depth 6
    return
}

if ($activeBuilds.Count -gt 0) {
    $ids = ($activeBuilds | ForEach-Object { $_["process_id"] }) -join ", "
    throw "Repo-local cargo/rustc build process already active ($ids). Run Status to inspect it; this script will not start a competing build."
}

$plan = Get-BuildPlan -BuildMode $Mode -TargetRoot $targetRoot -JobsOverride $Jobs
$releaseBinary = $plan.binary
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$buildStartedAt = Get-Date
$localBuildStamp = $buildStartedAt.ToString("yyyy-MM-ddTHH:mm:sszzz")
$plan.env_overrides["CODEX_LOCAL_BUILD_STAMP"] = $localBuildStamp
$logPath = Assert-UnderRoot -Path (Join-Path $RepoRoot "logs\local-codex-build-$($Mode.ToLowerInvariant())-$stamp.log") -Root $RepoRoot -Label "build log"
if ([System.IO.Path]::DirectorySeparatorChar -eq "\") {
    $plan.env_overrides["CARGO_BUILD_RUSTC_WRAPPER"] = Join-Path $RepoRoot "scripts\cargo-release-only-rustc-wrapper.exe"
}
else {
    $plan.env_overrides["CARGO_BUILD_RUSTC_WRAPPER"] = Join-Path $RepoRoot "scripts/cargo-release-only-rustc-wrapper"
}
if ($UseSccache) {
    $sccache = Get-Command sccache -ErrorAction SilentlyContinue
    if (-not $sccache) {
        throw "sccache was requested with -UseSccache, but it is not installed or not on PATH."
    }
    $plan.env_overrides["CODEX_CARGO_INNER_RUSTC_WRAPPER"] = $sccache.Source
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("SCCACHE_CACHE_SIZE", "Process")) -and
        [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("SCCACHE_CACHE_SIZE", "User")) -and
        [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("SCCACHE_CACHE_SIZE", "Machine"))) {
        $plan.env_overrides["SCCACHE_CACHE_SIZE"] = "2G"
    }
}

$buildRan = $false
if (-not $PSCmdlet.ShouldProcess($RepoRoot, "run $($plan.description)")) {
    [ordered]@{
        status = "planned"
        mode = $Mode
        cargo_args = $plan.cargo_args
        env_overrides = $plan.env_overrides
        timeout_seconds = $TimeoutSeconds
        log_path = $logPath
        release_binary = $releaseBinary
        deploy_skipped = [bool]$SkipDeploy
    } | ConvertTo-Json -Depth 8
    return
}

Ensure-ReleaseOnlyRustcWrapper -Root $RepoRoot

$profileState = Get-ReleaseProfileState -RepoRoot $RepoRoot
if ($profileState["stamp_exists"] -and -not $profileState["matches"]) {
    if (-not $ResetReleaseCacheOnProfileChange) {
        throw "Release profile/toolchain changed since the last successful local release build. To prevent another huge target/release generation, this script stopped before Cargo. Re-run with -ResetReleaseCacheOnProfileChange to remove target/release and rebuild one clean shared release cache. Stamp: $($profileState["stamp_path"])"
    }
    $reset = Invoke-ReleaseProfileCacheReset -RepoRoot $RepoRoot -ProfileState $profileState
    Write-Host ("Release profile changed; reset target/release ({0}, reclaimed {1:N1} MB)." -f $reset["status"], $reset["reclaimed_mb"])
}

# Pre-build planning, in this order:
#   1. Cross-mode cleanup: drop other-profile artifacts that just claim disk.
#   2. Disk-space defense: reclaim regeneratables, abort if still too low.
#   3. Wrapper-env sanity: surface obvious config breakage before the long
#      build, so the user catches "WIZARD_CODEX_CACHE_BRIDGE_PY missing"
#      style problems in 1 second instead of 30 minutes from now.
Invoke-CrossModeCleanup -RepoRoot $RepoRoot -ActiveMode $Mode
$releaseDepsCleanup = Invoke-ReleaseDepsOrphanCleanup -RepoRoot $RepoRoot
if ($releaseDepsCleanup["reclaimed_mb"] -gt 0) {
    Write-Host ("Pre-build cleanup: reclaimed {0:N1} MB from orphaned release deps artifacts." -f $releaseDepsCleanup["reclaimed_mb"])
}
Test-AndFreeDiskSpace -RepoRoot $RepoRoot -RequiredGB $DiskRequiredGB -WarnGB $DiskWarnGB
if ($envPath) {
    Test-WrapperEnvSanity -WrapperEnvPath $envPath
}

$exitCode = Invoke-CodexBuild -Root $RepoRoot -CargoArgs $plan.cargo_args -EnvOverrides $plan.env_overrides -LogPath $logPath -TimeoutSeconds $TimeoutSeconds
$buildRan = $true
if ($exitCode -ne 0) {
    Show-FailureLines -Path $logPath
    throw "cargo build failed with exit code $exitCode. Log: $logPath"
}
# Post-build housekeeping: free release/incremental (unused by the shared
# release deploy lane) so the next build starts with maximum headroom.
Invoke-PostBuildDiskCleanup -RepoRoot $RepoRoot -BuildMode $Mode
Write-ReleaseProfileStamp -RepoRoot $RepoRoot -ModeName $Mode -CargoArgs $plan.cargo_args | Out-Null

if ($buildRan -and -not (Test-Path -LiteralPath $releaseBinary)) {
    throw "Build did not produce $releaseBinary"
}

if ($buildRan -and -not $SkipVerify) {
    $versionOutput = & $releaseBinary --version
    $versionOutput | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "Built Codex exe failed --version with exit code $LASTEXITCODE"
    }
    if ($versionOutput -notmatch [regex]::Escape($localBuildStamp)) {
        throw "Built Codex exe --version did not include local build stamp $localBuildStamp"
    }
}

$deployResult = $null
if (-not $SkipDeploy) {
    $deployResult = Invoke-Deploy -ExePath $releaseBinary -ModeName $Mode -BuildStamp $localBuildStamp
}

[ordered]@{
    status = "ok"
    mode = $Mode
    cargo_args = $plan.cargo_args
    env_overrides = $plan.env_overrides
    local_build_stamp = $localBuildStamp
    log_path = $logPath
    release_binary = $releaseBinary
    deploy = $deployResult
} | ConvertTo-Json -Depth 8
